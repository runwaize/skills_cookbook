use crate::error::{RelayError, Result};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;

pub struct SkillDb {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub status: String,
    pub version: String,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
    pub synced_at: Option<String>,
}

impl SkillDb {
    pub fn open(base_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(base_dir)
            .map_err(|e| RelayError::Config(format!("create db dir: {}", e)))?;
        let db_path = base_dir.join("skills.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| RelayError::Config(format!("open skills db: {}", e)))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS skills (
                name TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'draft',
                version TEXT DEFAULT '0.1.0',
                tags TEXT DEFAULT '',
                description TEXT,
                updated_at TEXT NOT NULL,
                synced_at TEXT
            );
            CREATE TABLE IF NOT EXISTS usage_file_state (
                path TEXT PRIMARY KEY,
                mtime TEXT NOT NULL
            );",
        )
        .map_err(|e| RelayError::Config(format!("create skills table: {}", e)))?;

        // `usage_events` schema evolution: the old shape was
        // (skill, date, client, context) with PK (skill, date, client) — one
        // deduplicated "used at least once this day" row per triple. The new
        // shape adds `file_path` (the per-file cache identity) and `count`
        // (the real per-file invocation count), with PK
        // (skill, file_path, client) so re-scanning a file can cleanly
        // replace its own prior contribution. SQLite can't ALTER a PRIMARY
        // KEY in place, and this whole table is a derived/rebuildable cache
        // (never user-authored) — so rather than migrate old rows, just drop
        // and recreate the table whenever the OLD column shape is detected.
        // The `USAGE_CACHE_VERSION` bump alongside this change makes
        // `sync_roots`'s existing version-mismatch auto-invalidation clear
        // `usage_file_state` too, so a full clean resync naturally follows.
        {
            let mut stmt = conn
                .prepare("PRAGMA table_info(usage_events)")
                .map_err(|e| RelayError::Config(format!("inspect usage_events: {}", e)))?;
            let columns: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| RelayError::Config(format!("inspect usage_events: {}", e)))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| RelayError::Config(format!("inspect usage_events: {}", e)))?;
            drop(stmt);

            let has_old_shape = !columns.is_empty()
                && (!columns.iter().any(|c| c == "file_path")
                    || !columns.iter().any(|c| c == "count"));
            if has_old_shape {
                conn.execute_batch("DROP TABLE IF EXISTS usage_events;")
                    .map_err(|e| RelayError::Config(format!("drop stale usage_events: {}", e)))?;
            }
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_events (
                skill TEXT NOT NULL,
                file_path TEXT NOT NULL,
                date TEXT NOT NULL,
                client TEXT NOT NULL,
                context TEXT NOT NULL DEFAULT '',
                count INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (skill, file_path, client)
            );",
        )
        .map_err(|e| RelayError::Config(format!("create usage_events table: {}", e)))?;

        // Migration: add `source` and `created_at` columns for pre-existing DBs.
        // ALTER TABLE ADD COLUMN is safe/non-destructive; ignore only the
        // "duplicate column name" error (column already added by a prior run).
        for stmt in [
            "ALTER TABLE skills ADD COLUMN source TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE skills ADD COLUMN created_at TEXT NOT NULL DEFAULT ''",
        ] {
            if let Err(e) = conn.execute(stmt, []) {
                if !e.to_string().contains("duplicate column name") {
                    return Err(RelayError::Config(format!("migrate skills table: {}", e)));
                }
            }
        }

        // One-time backfill: rows created before this feature shipped have a
        // blank created_at; give them a sensible installed-date fallback
        // (their last known update time). Safe to run unconditionally — it's
        // a no-op once created_at is populated.
        conn.execute(
            "UPDATE skills SET created_at = updated_at WHERE created_at = ''",
            [],
        )
        .map_err(|e| RelayError::Config(format!("backfill created_at: {}", e)))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get(&self, name: &str) -> Result<Option<SkillMeta>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT name, status, version, tags, description, updated_at, synced_at,
                        source, created_at
                 FROM skills WHERE name = ?1",
            )
            .map_err(|e| RelayError::Internal(e.to_string()))?;

        let result = stmt
            .query_row(rusqlite::params![name], |row| {
                let tags_str: String = row.get(3)?;
                Ok(SkillMeta {
                    name: row.get(0)?,
                    status: row.get(1)?,
                    version: row.get(2)?,
                    tags: parse_tags(&tags_str),
                    description: row.get(4)?,
                    updated_at: row.get(5)?,
                    synced_at: row.get(6)?,
                    source: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .optional()
            .map_err(|e| RelayError::Internal(e.to_string()))?;

        Ok(result)
    }

    pub fn upsert(
        &self,
        name: &str,
        status: &str,
        version: &str,
        tags: &[String],
        description: Option<&str>,
        source: &str,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let tags_str = tags.join(",");
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO skills (name, status, version, tags, description, source, updated_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(name) DO UPDATE SET
                status = excluded.status,
                version = excluded.version,
                tags = excluded.tags,
                description = excluded.description,
                source = excluded.source,
                updated_at = excluded.updated_at",
            rusqlite::params![name, status, version, tags_str, description, source, now],
        )
        .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }

    pub fn ensure_exists(&self, name: &str) -> Result<SkillMeta> {
        if let Some(meta) = self.get(name)? {
            return Ok(meta);
        }
        self.upsert(name, "draft", "0.1.0", &[], None, "existing")?;
        self.get(name)?
            .ok_or_else(|| RelayError::Internal("failed to create skill meta".to_string()))
    }

    /// Record provenance for a skill without touching its other metadata.
    /// Ensures a row exists first (e.g. for skills just copied/adopted in).
    pub fn set_source(&self, name: &str, source: &str) -> Result<()> {
        self.ensure_exists(name)?;
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE skills SET source = ?1, updated_at = ?2 WHERE name = ?3",
            rusqlite::params![source, now, name],
        )
        .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Seed a skill's description without touching its other metadata. Ensures a
    /// row exists first, same as `set_source` -- used right after import to carry
    /// over the SKILL.md frontmatter description instead of leaving it blank.
    pub fn set_description(&self, name: &str, description: &str) -> Result<()> {
        self.ensure_exists(name)?;
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE skills SET description = ?1, updated_at = ?2 WHERE name = ?3",
            rusqlite::params![description, now, name],
        )
        .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn list_all(&self) -> Result<Vec<SkillMeta>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT name, status, version, tags, description, updated_at, synced_at,
                        source, created_at
                 FROM skills ORDER BY name",
            )
            .map_err(|e| RelayError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let tags_str: String = row.get(3)?;
                Ok(SkillMeta {
                    name: row.get(0)?,
                    status: row.get(1)?,
                    version: row.get(2)?,
                    tags: parse_tags(&tags_str),
                    description: row.get(4)?,
                    updated_at: row.get(5)?,
                    synced_at: row.get(6)?,
                    source: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| RelayError::Internal(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| RelayError::Internal(e.to_string()))?);
        }
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn mark_synced(&self, name: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE skills SET synced_at = ?1 WHERE name = ?2",
            rusqlite::params![now, name],
        )
        .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete(&self, name: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM skills WHERE name = ?1",
            rusqlite::params![name],
        )
        .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Look up the cached mtime for a transcript file, if any.
    pub fn usage_file_mtime(&self, path: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT mtime FROM usage_file_state WHERE path = ?1")
            .map_err(|e| RelayError::Internal(e.to_string()))?;

        let result = stmt
            .query_row(rusqlite::params![path], |row| row.get::<_, String>(0))
            .optional_string()
            .map_err(|e| RelayError::Internal(e.to_string()))?;

        Ok(result)
    }

    /// Persist many (path, mtime, events) updates in a single SQLite
    /// transaction. Used for a full usage-cache sync, where scanning
    /// thousands of transcript files can each produce an update — doing
    /// each one as its own auto-committed statement (the per-file/per-event
    /// methods above) means one fsync per statement, which is unusably slow
    /// at that volume. Batching into one transaction reduces that to a
    /// single commit for the whole sync.
    ///
    /// For each `(path, mtime, events)` update, first deletes ANY rows this
    /// exact file previously contributed (`WHERE file_path = ?1`), then
    /// inserts the fresh per-skill rows — this correctly replaces a file's
    /// own prior contribution (handles both "file changed, different skills
    /// now" and re-running an unchanged sync) rather than ever adding to it.
    /// Since `extract_events_from_file` guarantees at most one event per
    /// (skill, file), the insert can never collide with itself within the
    /// same update, so a plain `INSERT` (not `OR IGNORE`) is used — a
    /// collision here would indicate a real bug upstream, and should surface
    /// as an error rather than be silently swallowed.
    pub fn sync_usage_batch(
        &self,
        updates: &[(
            String,
            String,
            std::collections::HashSet<crate::usage::UsageEvent>,
        )],
    ) -> Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn
            .transaction()
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        for (path, mtime, events) in updates {
            tx.execute(
                "DELETE FROM usage_events WHERE file_path = ?1",
                rusqlite::params![path],
            )
            .map_err(|e| RelayError::Internal(e.to_string()))?;

            for event in events {
                tx.execute(
                    "INSERT INTO usage_events (skill, file_path, date, client, context, count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        event.skill,
                        path,
                        event.date,
                        event.client,
                        event.context,
                        event.count as i64
                    ],
                )
                .map_err(|e| RelayError::Internal(e.to_string()))?;
            }
            tx.execute(
                "INSERT INTO usage_file_state (path, mtime) VALUES (?1, ?2)
                 ON CONFLICT(path) DO UPDATE SET mtime = excluded.mtime",
                rusqlite::params![path, mtime],
            )
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Query cached usage events on or after `since_date` (ISO `YYYY-MM-DD`).
    /// Returns the RAW per-file rows (skill, date, client, context, count) —
    /// no summing/grouping is done here. Aggregating across files for a
    /// skill's total (or across skills for a daily total) is the caller's
    /// job.
    pub fn query_usage_events(&self, since_date: &str) -> Result<Vec<crate::usage::UsageEvent>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT skill, date, client, context, count FROM usage_events WHERE date >= ?1",
            )
            .map_err(|e| RelayError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![since_date], |row| {
                let count: i64 = row.get(4)?;
                Ok(crate::usage::UsageEvent {
                    skill: row.get(0)?,
                    date: row.get(1)?,
                    client: row.get(2)?,
                    context: row.get(3)?,
                    count: count.max(0) as usize,
                })
            })
            .map_err(|e| RelayError::Internal(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| RelayError::Internal(e.to_string()))?);
        }
        Ok(result)
    }

    /// Record the usage-cache schema version marker, reusing
    /// `usage_file_state`'s `mtime` column (keyed under a reserved sentinel
    /// path) to hold a plain version string instead of an actual file mtime.
    /// Read back via the existing [`Self::usage_file_mtime`] lookup.
    pub fn set_usage_cache_version(&self, version: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO usage_file_state (path, mtime) VALUES (?1, ?2)
             ON CONFLICT(path) DO UPDATE SET mtime = excluded.mtime",
            rusqlite::params![crate::usage::USAGE_CACHE_VERSION_SENTINEL_PATH, version],
        )
        .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Wipe both usage cache tables — used by the manual full resync path.
    pub fn clear_usage_cache(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch("DELETE FROM usage_events; DELETE FROM usage_file_state;")
            .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }
}

fn parse_tags(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

trait OptionalRow {
    fn optional(self) -> std::result::Result<Option<SkillMeta>, rusqlite::Error>;
}

impl OptionalRow for std::result::Result<SkillMeta, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<SkillMeta>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

trait OptionalStringRow {
    fn optional_string(self) -> std::result::Result<Option<String>, rusqlite::Error>;
}

impl OptionalStringRow for std::result::Result<String, rusqlite::Error> {
    fn optional_string(self) -> std::result::Result<Option<String>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
