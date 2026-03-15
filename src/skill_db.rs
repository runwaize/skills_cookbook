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
            );",
        )
        .map_err(|e| RelayError::Config(format!("create skills table: {}", e)))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get(&self, name: &str) -> Result<Option<SkillMeta>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT name, status, version, tags, description, updated_at, synced_at
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
    ) -> Result<()> {
        let conn = self.conn.lock();
        let tags_str = tags.join(",");
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO skills (name, status, version, tags, description, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(name) DO UPDATE SET
                status = excluded.status,
                version = excluded.version,
                tags = excluded.tags,
                description = excluded.description,
                updated_at = excluded.updated_at",
            rusqlite::params![name, status, version, tags_str, description, now],
        )
        .map_err(|e| RelayError::Internal(e.to_string()))?;
        Ok(())
    }

    pub fn ensure_exists(&self, name: &str) -> Result<SkillMeta> {
        if let Some(meta) = self.get(name)? {
            return Ok(meta);
        }
        self.upsert(name, "draft", "0.1.0", &[], None)?;
        self.get(name)?
            .ok_or_else(|| RelayError::Internal("failed to create skill meta".to_string()))
    }

    #[allow(dead_code)]
    pub fn list_all(&self) -> Result<Vec<SkillMeta>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT name, status, version, tags, description, updated_at, synced_at
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
        conn.execute("DELETE FROM skills WHERE name = ?1", rusqlite::params![name])
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
