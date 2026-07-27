use crate::migrations;
use rusqlite::{Connection, Result};
use std::sync::Mutex;
use tauri::Manager;

pub struct DbState(pub Mutex<Connection>);

pub fn init_db(app_handle: &tauri::AppHandle) -> Result<Connection> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");

    std::fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
    let db_path = app_dir.join("bloc.db");

    let conn = Connection::open(db_path)?;

    // Foreign key enforcement is connection-level in SQLite; must be set on every open.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    migrations::run(&conn)?;

    Ok(conn)
}
