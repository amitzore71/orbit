use super::*;

#[test]
fn legacy_appearance_defaults_and_curated_preferences_roundtrip_without_migration() {
    use focus_core::{Accent, Density, MotionLevel};
    let mut store = Store::connection(Connection::open_in_memory().unwrap()).unwrap();
    store
        .db
        .execute(
            "INSERT OR REPLACE INTO meta(key,value) VALUES('settings', ?)",
            [r#"{"appearance":2,"focus_minutes":40,"reduced_motion":true}"#],
        )
        .unwrap();
    let legacy = store.load().unwrap().settings;
    assert_eq!(legacy.appearance, 2);
    assert_eq!(legacy.focus_minutes, 40);
    assert_eq!(legacy.accent, Accent::Coral);
    assert_eq!(legacy.density, Density::Comfortable);
    assert_eq!(legacy.effective_motion(), MotionLevel::Reduced);
    let mut preferences = Settings {
        accent: Accent::SlateBlue,
        density: Density::Compact,
        motion_level: Some(MotionLevel::Off),
        ..legacy
    };
    preferences.normalize_appearance();
    store
        .process(Operation::Settings(preferences.clone()))
        .unwrap();
    assert_eq!(store.load().unwrap().settings, preferences);
    assert_eq!(
        store
            .db
            .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
            .unwrap(),
        4
    );
    for invalid in [
        r#"{"accent":"neon"}"#,
        r#"{"density":"tiny"}"#,
        r#"{"motion_level":"fast"}"#,
    ] {
        assert!(serde_json::from_str::<Settings>(invalid).is_err());
    }
}

fn directory() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts/storage-product")
        .join(format!("{}-{}", std::process::id(), unique_stamp()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn planning_reorder_restore_and_history_use_stable_task_ids() {
    let mut store = Store::connection(Connection::open_in_memory().unwrap()).unwrap();
    for title in ["Draft 日本語", "Review हिंदी", "Backlog"] {
        store
            .process(Operation::AddTask {
                title: title.into(),
                estimate: 2,
            })
            .unwrap();
    }
    store
        .process(Operation::PlanTask { id: 1, today: true })
        .unwrap();
    store
        .process(Operation::PlanTask { id: 2, today: true })
        .unwrap();
    store
        .process(Operation::MoveTask {
            id: 2,
            direction: -1,
        })
        .unwrap();
    let rows = Store::all_tasks_in(&store.db).unwrap();
    assert_eq!(
        rows.iter()
            .filter(|t| t.planned_for == Some(local_date()))
            .map(|t| t.id)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    store
        .db
        .execute("INSERT INTO sessions VALUES(77,1,1,100)", [])
        .unwrap();
    store.process(Operation::DeleteTask(1)).unwrap();
    assert!(
        Store::all_tasks_in(&store.db)
            .unwrap()
            .iter()
            .find(|t| t.id == 1)
            .unwrap()
            .deleted
    );
    store.process(Operation::RestoreTask(1)).unwrap();
    let restored = store
        .tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.id == 1)
        .unwrap();
    assert_eq!(restored.completed, 1);
    assert_eq!(restored.title, "Draft 日本語");
    assert!(restored.planned_for.is_none());
}

#[test]
fn backup_restore_round_trip_is_paused_and_keeps_this_machines_startup_choice() {
    let dir = directory();
    let mut store = Store::open(dir.join("orbit.db")).unwrap();
    store
        .process(Operation::CreateTask {
            title: "Saved work".into(),
            estimate: 3,
            today: true,
        })
        .unwrap();
    let mut timer = focus_core::Timer::new(Settings::default());
    timer.dispatch(
        focus_core::Command::StartTask(1),
        focus_core::TimePoint {
            awake_ms: 0,
            utc_ms: 0,
        },
    );
    store
        .process(Operation::Transition {
            effects: vec![],
            checkpoint: timer.checkpoint(focus_core::TimePoint {
                awake_ms: 0,
                utc_ms: 0,
            }),
        })
        .unwrap();
    let backup = dir.join("manual.db");
    store.process(Operation::Backup(backup.clone())).unwrap();
    assert!(
        store.process(Operation::Backup(backup.clone())).is_err(),
        "Existing backups must never be overwritten"
    );
    store.process(Operation::DeleteTask(1)).unwrap();
    store
        .process(Operation::Settings(Settings {
            autostart: true,
            ..Default::default()
        }))
        .unwrap();
    let loaded = match store.process(Operation::Restore(backup)).unwrap() {
        Response::Restored(loaded) => loaded,
        _ => panic!("restore response"),
    };
    assert_eq!(loaded.tasks.len(), 1);
    assert!(!loaded.tasks[0].deleted);
    assert!(loaded.settings.autostart);
    assert!(!loaded.checkpoint.unwrap().running);
    assert!(std::fs::read_dir(dir.join("backups")).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("orbit-before-restore-")
    }));
}

#[test]
fn invalid_or_future_restore_preserves_the_active_database_and_retry_queue() {
    let dir = directory();
    let mut store = Store::open(dir.join("orbit.db")).unwrap();
    store
        .process(Operation::AddTask {
            title: "Keep me".into(),
            estimate: 1,
        })
        .unwrap();
    let bad = dir.join("future.db");
    let db = Connection::open(&bad).unwrap();
    db.execute_batch("PRAGMA user_version=99").unwrap();
    drop(db);
    assert!(store.process(Operation::Restore(bad)).is_err());
    assert!(store.pending.is_empty());
    assert_eq!(store.tasks().unwrap()[0].title, "Keep me");
    assert_eq!(
        store
            .db
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn schema_three_upgrade_backs_up_before_migration_and_leaves_backlog_unplanned() {
    let dir = directory();
    let path = dir.join("legacy.db");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE tasks(id INTEGER PRIMARY KEY AUTOINCREMENT,title TEXT NOT NULL,estimate INTEGER NOT NULL,done INTEGER NOT NULL DEFAULT 0,deleted INTEGER NOT NULL DEFAULT 0);
        CREATE TABLE sessions(id INTEGER PRIMARY KEY,task_id INTEGER,completed INTEGER NOT NULL,ended_ms INTEGER NOT NULL);
        CREATE TABLE intervals(id INTEGER PRIMARY KEY,session_id INTEGER NOT NULL,task_id INTEGER,start_ms INTEGER NOT NULL,end_ms INTEGER NOT NULL,active_ms INTEGER NOT NULL);
        CREATE INDEX intervals_end ON intervals(end_ms);
        CREATE INDEX intervals_long_start ON intervals(start_ms) WHERE end_ms-start_ms>10800000;
        INSERT INTO tasks(title,estimate) VALUES('Existing task',2);
        INSERT INTO meta VALUES('settings','{\"focus_minutes\":30}');
        PRAGMA user_version=3;").unwrap();
    drop(db);
    let store = Store::open(path).unwrap();
    assert!(store.tasks().unwrap()[0].planned_for.is_none());
    assert_eq!(store.load().unwrap().settings.appearance, 1);
    assert_eq!(store.load().unwrap().settings.hotkey_skip, "Ctrl+Alt+N");
    let backups: Vec<_> = std::fs::read_dir(dir.join("backups"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = Connection::open(backups[0].path()).unwrap();
    assert_eq!(
        backup
            .pragma_query_value::<u32, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        3
    );
    assert_eq!(
        backup
            .query_row("SELECT title FROM tasks", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Existing task"
    );
}

#[test]
fn custom_report_and_csv_reconcile_with_unassigned_and_deleted_history() {
    let dir = directory();
    let store = Store::open(dir.join("orbit.db")).unwrap();
    let date = Local::now().date_naive();
    let start = local_midnight(date).unwrap();
    store
        .db
        .execute(
            "INSERT INTO tasks(title,estimate,deleted) VALUES('=Danger, 日本語',1,1)",
            [],
        )
        .unwrap();
    store.db.execute("INSERT INTO intervals(session_id,task_id,start_ms,end_ms,active_ms) VALUES(1,1,?1,?2,60000)",params![start,start+60_000]).unwrap();
    store
        .db
        .execute(
            "INSERT INTO intervals(session_id,start_ms,end_ms,active_ms) VALUES(2,?1,?2,30000)",
            params![start + 60_000, start + 90_000],
        )
        .unwrap();
    let report = store
        .custom_report(&date.to_string(), &date.to_string())
        .unwrap();
    assert_eq!(report.active_ms, 90_000);
    assert_eq!(report.tasks.len(), 2);
    let path = dir.join("report.csv");
    store
        .export_report(
            path.clone(),
            Period::Day,
            0,
            date.to_string(),
            date.to_string(),
        )
        .unwrap();
    let csv = std::fs::read_to_string(path).unwrap();
    assert!(csv.contains("'=Danger, 日本語"));
    assert!(csv.contains("90000,0"));
    assert!(store.custom_report("2026-10-20", "2026-10-19").is_err());
}
