#[cfg(test)]
mod tests {
    use crate::skill_db::SkillDb;
    use tempfile::TempDir;

    #[test]
    fn test_upsert_preserves_created_at_across_updates() {
        let dir = TempDir::new().unwrap();
        let db = SkillDb::open(dir.path()).unwrap();

        db.upsert("my-skill", "draft", "0.1.0", &[], None, "created")
            .unwrap();
        let first = db.get("my-skill").unwrap().unwrap();
        assert_eq!(first.source, "created");
        assert!(!first.created_at.is_empty());

        // Small delay isn't required: even if timestamps collide, created_at
        // must remain identical to the first insert's value.
        db.upsert("my-skill", "published", "0.2.0", &[], None, "adapted")
            .unwrap();
        let second = db.get("my-skill").unwrap().unwrap();

        assert_eq!(second.created_at, first.created_at);
        assert_eq!(second.source, "adapted");
        assert_eq!(second.status, "published");
        assert_eq!(second.version, "0.2.0");
    }

    #[test]
    fn test_set_source_creates_row_if_missing() {
        let dir = TempDir::new().unwrap();
        let db = SkillDb::open(dir.path()).unwrap();

        assert!(db.get("new-skill").unwrap().is_none());

        db.set_source("new-skill", "downloaded").unwrap();

        let meta = db.get("new-skill").unwrap().unwrap();
        assert_eq!(meta.source, "downloaded");
        assert!(!meta.created_at.is_empty());
    }

    #[test]
    fn test_set_source_updates_existing_row_without_touching_other_fields() {
        let dir = TempDir::new().unwrap();
        let db = SkillDb::open(dir.path()).unwrap();

        db.upsert(
            "existing-skill",
            "published",
            "1.0.0",
            &["a".to_string(), "b".to_string()],
            Some("a description"),
            "created",
        )
        .unwrap();
        let before = db.get("existing-skill").unwrap().unwrap();

        db.set_source("existing-skill", "imported").unwrap();
        let after = db.get("existing-skill").unwrap().unwrap();

        assert_eq!(after.source, "imported");
        assert_eq!(after.created_at, before.created_at);
        assert_eq!(after.status, "published");
        assert_eq!(after.version, "1.0.0");
        assert_eq!(after.tags, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(after.description, Some("a description".to_string()));
    }

    #[test]
    fn test_migration_alter_table_is_idempotent() {
        let dir = TempDir::new().unwrap();

        // First open creates the table and runs the migration.
        {
            let db = SkillDb::open(dir.path()).unwrap();
            db.upsert("skill-one", "draft", "0.1.0", &[], None, "created")
                .unwrap();
        }

        // Second open on the same tempdir must not error even though the
        // `source`/`created_at` columns already exist from the first open.
        let db2 = SkillDb::open(dir.path()).unwrap();
        let meta = db2.get("skill-one").unwrap().unwrap();
        assert_eq!(meta.source, "created");
    }

    /// Simulates a database created before this feature shipped: a `skills`
    /// table with only the original 7 columns and a real row in it. Opening
    /// it with the new `SkillDb::open` must migrate the schema (ALTER TABLE)
    /// and backfill `created_at` from `updated_at` for that pre-existing row.
    #[test]
    fn test_migration_backfills_created_at_for_legacy_rows() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("skills.db");

        let legacy_updated_at = "2024-01-01T00:00:00+00:00";
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
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
            .unwrap();
            conn.execute(
                "INSERT INTO skills (name, status, version, tags, description, updated_at)
                 VALUES ('legacy-skill', 'published', '2.0.0', 'x,y', 'legacy desc', ?1)",
                rusqlite::params![legacy_updated_at],
            )
            .unwrap();
        }

        // Opening via SkillDb::open must ALTER TABLE to add the new columns
        // and backfill created_at from updated_at for the legacy row.
        let db = SkillDb::open(dir.path()).unwrap();
        let meta = db.get("legacy-skill").unwrap().unwrap();

        assert_eq!(meta.created_at, legacy_updated_at);
        assert_eq!(meta.source, "");
        assert_eq!(meta.status, "published");
        assert_eq!(meta.version, "2.0.0");
    }

    /// Simulates a database created before the per-file `count`/`file_path`
    /// usage-cache redesign: an `usage_events` table with only the OLD
    /// 4-column shape (skill, date, client, context; PK skill,date,client)
    /// and a real row in it. `SkillDb::open` must detect the missing
    /// `file_path`/`count` columns, drop the stale table, and recreate it in
    /// the new shape — this data is a disposable derived cache, so no
    /// migration of the old row is expected, just a clean empty table that
    /// the normal cache-version-mismatch resync path repopulates from
    /// scratch. The key proof is that `query_usage_events` runs without
    /// erroring (it selects the new `count` column) and returns nothing for
    /// the never-migrated legacy row.
    #[test]
    fn old_shape_usage_events_table_is_dropped_and_recreated_in_new_shape() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("skills.db");

        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS usage_events (
                    skill TEXT NOT NULL,
                    date TEXT NOT NULL,
                    client TEXT NOT NULL,
                    context TEXT NOT NULL DEFAULT '',
                    PRIMARY KEY (skill, date, client)
                );",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO usage_events (skill, date, client, context)
                 VALUES ('legacy-skill', '2024-01-01', 'claude_code', '')",
                [],
            )
            .unwrap();
        }

        // Opening via SkillDb::open must detect the old column shape, drop
        // the table, and recreate it with `file_path`/`count` — without
        // erroring, and without carrying the legacy row forward.
        let db = SkillDb::open(dir.path()).unwrap();
        let events = db
            .query_usage_events("2000-01-01")
            .expect("query on recreated table must not error");

        assert!(
            events.is_empty(),
            "old-shape row must not survive the drop+recreate migration"
        );
    }
}
