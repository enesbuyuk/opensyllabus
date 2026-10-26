use opensyllabus::{config::Config, repositories::db, services::seed};

#[tokio::test]
async fn migrations_apply_and_configure_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config::for_tests(dir.path());
    let pool = db::connect(&cfg.database_path).await.unwrap();
    db::migrate(&pool).await.unwrap();
    // Re-running is a no-op.
    db::migrate(&pool).await.unwrap();

    let fk: i64 = sqlx::query_scalar("PRAGMA foreign_keys").fetch_one(&pool).await.unwrap();
    assert_eq!(fk, 1);
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode").fetch_one(&pool).await.unwrap();
    assert_eq!(mode.to_lowercase(), "wal");
    let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table'").fetch_all(&pool).await.unwrap();
    for t in [
        "users",
        "sessions",
        "courses",
        "course_enrollments",
        "modules",
        "weeks",
        "lessons",
        "lesson_progress",
        "assignments",
        "assignment_rubrics",
        "submissions",
        "submission_revisions",
        "submission_attachments",
        "grades",
        "feedback",
        "resources",
        "announcements",
        "notifications",
        "calendar_events",
        "audit_logs",
        "files",
    ] {
        assert!(tables.contains(&t.to_string()), "missing table {t}");
    }
    // Foreign keys are enforced.
    let err = sqlx::query("INSERT INTO course_enrollments (id, course_id, user_id, enrolled_at) VALUES ('x', 'nope', 'nope', 'now')")
        .execute(&pool)
        .await;
    assert!(err.is_err());
}

#[tokio::test]
async fn initial_course_seed_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config::for_tests(dir.path());
    let pool = db::connect(&cfg.database_path).await.unwrap();
    db::migrate(&pool).await.unwrap();
    seed::seed_initial_course(&pool).await.unwrap();
    seed::seed_initial_course(&pool).await.unwrap();
    let courses: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM courses").fetch_one(&pool).await.unwrap();
    let weeks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM weeks").fetch_one(&pool).await.unwrap();
    let modules: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM modules").fetch_one(&pool).await.unwrap();
    let lessons: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lessons").fetch_one(&pool).await.unwrap();
    assert_eq!((courses, weeks, modules, lessons), (1, 48, 8, 3));
    let published: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM weeks WHERE status = 'published'").fetch_one(&pool).await.unwrap();
    assert_eq!(published, 0, "seeded weeks start as drafts");
}

#[tokio::test]
async fn usernames_are_backfilled_from_unique_email_local_parts() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config::for_tests(dir.path());
    let pool = db::connect(&cfg.database_path).await.unwrap();
    // Schema as it was before 0002 (same 0001 file, so the checksum matches), with accounts created then.
    let only_first = dir.path().join("migrations-0001");
    std::fs::create_dir_all(&only_first).unwrap();
    std::fs::copy("migrations/0001_initial_schema.sql", only_first.join("0001_initial_schema.sql")).unwrap();
    sqlx::migrate::Migrator::new(only_first.as_path()).await.unwrap().run(&pool).await.unwrap();
    for (id, email) in [("1", "Enes@a.com"), ("2", "dup@a.com"), ("3", "dup@b.com"), ("4", "x@a.com"), ("5", "has+plus@a.com")] {
        sqlx::query(
            "INSERT INTO users (id, email, name, role, password_hash, created_at, updated_at) VALUES (?, ?, 'U', 'student', 'h', '', '')",
        )
        .bind(id)
        .bind(email)
        .execute(&pool)
        .await
        .unwrap();
    }
    db::migrate(&pool).await.unwrap();
    let rows: Vec<(String, Option<String>)> = sqlx::query_as("SELECT id, username FROM users ORDER BY id").fetch_all(&pool).await.unwrap();
    let names: Vec<Option<&str>> = rows.iter().map(|(_, u)| u.as_deref()).collect();
    assert_eq!(names, [Some("enes"), None, None, None, None]);
}
