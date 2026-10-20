use super::*;
use std::{
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(super) fn unique_stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

pub(super) fn backup_path(active: &Path, kind: &str) -> Result<PathBuf> {
    let directory = active
        .parent()
        .ok_or("Database directory unavailable")?
        .join("backups");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    Ok(directory.join(format!("orbit-{kind}-{}.db", unique_stamp())))
}

pub(super) fn copy_database(source: &Connection, target: &mut Connection) -> Result<()> {
    let backup = rusqlite::backup::Backup::new(source, target).map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match backup.step(128).map_err(|e| e.to_string())? {
            rusqlite::backup::StepResult::Done => return Ok(()),
            _ if started.elapsed() > Duration::from_secs(30) => {
                return Err("Backup timed out; original data remains available.".into());
            }
            rusqlite::backup::StepResult::Busy | rusqlite::backup::StepResult::Locked => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => {}
        }
    }
}

pub(super) fn validate_backup(db: &Connection) -> Result<()> {
    let version: u32 = db
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if !(1..=4).contains(&version) {
        return Err(
            "Unsupported backup schema. Use a backup from this or an earlier Orbit version.".into(),
        );
    }
    let integrity: String = db
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err(format!("Backup failed integrity check: {integrity}"));
    }
    for query in [
        "SELECT key,value FROM meta LIMIT 0",
        "SELECT id,title,estimate,done,deleted FROM tasks LIMIT 0",
        "SELECT id,task_id,completed,ended_ms FROM sessions LIMIT 0",
        "SELECT id,session_id,task_id,start_ms,end_ms,active_ms FROM intervals LIMIT 0",
    ] {
        db.prepare(query)
            .map_err(|e| format!("Invalid Orbit backup: {e}"))?;
    }
    Ok(())
}

pub(super) fn snapshot(source: &Connection, destination: &Path) -> Result<()> {
    // Never overwrite an existing user-selected backup or an active database.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    let result = (|| {
        let mut target = Connection::open(destination).map_err(|e| e.to_string())?;
        copy_database(source, &mut target)?;
        validate_backup(&target)?;
        drop(target);
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(destination)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(destination);
    }
    result
}

pub(super) fn prune_upgrade_backups(active: &Path) {
    let Some(parent) = active.parent() else {
        return;
    };
    let directory = parent.join("backups");
    let Ok(entries) = std::fs::read_dir(&directory) else {
        return;
    };
    let mut files: Vec<_> = entries
        .flatten()
        .filter(|e| {
            e.file_type().is_ok_and(|t| t.is_file())
                && e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("orbit-upgrade-") && n.ends_with(".db"))
        })
        .map(|e| e.path())
        .collect();
    files.sort();
    let excess = files.len().saturating_sub(3);
    for file in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(file);
    }
}
