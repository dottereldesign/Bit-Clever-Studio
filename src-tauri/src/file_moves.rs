//! Durable intent for owned-file moves across SQLite and filesystem boundaries.
use crate::{
    content::hash_file,
    db::{self, err, Result},
};
use rusqlite::{params, Connection, TransactionBehavior};
use std::{fs, path::Path};

const RECOVERY_WARNING: &str = "Saved your changes, but library files need recovery. ";

pub fn recover_on_startup(path: &Path) -> Result<()> {
    match reconcile(path) {
        Ok(()) => Ok(()),
        Err(error) => db::set_setting(
            &db::connect(path)?,
            "maintenance_warning",
            &format!("{RECOVERY_WARNING}{error} Use Rescan library to retry."),
        ),
    }
}

pub fn init(db: &Connection) -> Result<()> {
    crate::import_copies::init(db)?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS file_move_journal(item_id INTEGER PRIMARY KEY REFERENCES items(id),source TEXT NOT NULL,destination TEXT NOT NULL,sha256 TEXT NOT NULL);").map_err(err)
}
pub fn record(db: &Connection, id: i64, source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        return Err("The destination already exists.".into());
    }
    let hash = hash_file(source)?;
    db.execute(
        "INSERT INTO file_move_journal VALUES(?,?,?,?)",
        params![
            id,
            source.to_string_lossy(),
            destination.to_string_lossy(),
            hash
        ],
    )
    .map_err(err)?;
    Ok(())
}
pub fn finish(db: &Connection, id: i64) -> Result<()> {
    db.execute("DELETE FROM file_move_journal WHERE item_id=?", [id])
        .map_err(err)?;
    Ok(())
}

// Publish without overwriting an existing file. On one volume a hard link avoids
// copying large media. Across volumes a verified temporary copy is published first.
pub fn move_file(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        return Err("The destination already exists.".into());
    }
    #[cfg(test)]
    let force_copy = std::env::var("DESIGN_TEST_FORCE_COPY").ok().as_deref() == Some("1");
    #[cfg(not(test))]
    let force_copy = false;
    if force_copy || fs::hard_link(source, destination).is_err() {
        let mut input = fs::File::open(source).map_err(err)?;
        let mut output = tempfile::NamedTempFile::new_in(
            destination.parent().ok_or("Missing destination folder")?,
        )
        .map_err(err)?;
        std::io::copy(&mut input, &mut output).map_err(err)?;
        output.as_file().sync_all().map_err(err)?;
        if hash_file(source)? != hash_file(output.path())? {
            return Err("Copy verification failed. The original was kept.".into());
        }
        output.persist_noclobber(destination).map_err(err)?;
    }
    checkpoint("move.after_publish");
    fs::remove_file(source).map_err(err)
}

// The journal row disappears in the SAME transaction as the metadata commit.
// Any surviving row therefore means: return the filesystem to the old DB state.
pub fn reconcile(path: &Path) -> Result<()> {
    let mut db = db::connect(path)?;
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(err)?;
    let moves = {
        let mut q = tx
            .prepare("SELECT item_id,source,destination,sha256 FROM file_move_journal")
            .map_err(err)?;
        let rows = q
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(err)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)?
    };
    for (id, source, destination, hash) in moves {
        let source = Path::new(&source);
        let destination = Path::new(&destination);
        if source.is_file() {
            if hash_file(source)? != hash {
                return Err("An interrupted file move has a changed original. Both locations were kept; resolve the conflict before retrying.".into());
            }
            if destination.exists() {
                if !destination.is_file() || hash_file(destination)? != hash {
                    return Err("An interrupted file move has a conflicting destination. Both locations were kept; resolve the conflict before retrying.".into());
                }
                // A crash between publication and source removal left two verified copies.
                fs::remove_file(destination).map_err(err)?;
            }
        } else if !source.exists() && destination.is_file() {
            if hash_file(destination)? != hash {
                return Err("An interrupted file move has a changed recovery copy. It was kept; resolve the conflict before retrying.".into());
            }
            move_file(destination, source)?;
        } else {
            return Err("An interrupted file move is unavailable. Reconnect its drive and rescan or retry; no recovery record was discarded.".into());
        }
        finish(&tx, id)?;
    }
    crate::import_copies::reconcile(&tx)?;
    tx.commit().map_err(err)?;
    if db::setting(&db, "maintenance_warning")
        .is_some_and(|warning| warning.starts_with(RECOVERY_WARNING))
    {
        db::set_setting(&db, "maintenance_warning", "")?;
    }
    Ok(())
}
pub fn rollback_error(path: &Path, original: String) -> String {
    match reconcile(path) {
        Ok(()) => original,
        Err(recovery) => format!("{original}\nRecovery pending: {recovery}"),
    }
}

#[cfg(test)]
pub fn checkpoint(point: &str) {
    if std::env::var("DESIGN_TEST_CRASH_POINT").ok().as_deref() == Some(point) {
        std::process::exit(86);
    }
}
#[cfg(not(test))]
pub fn checkpoint(_: &str) {}
