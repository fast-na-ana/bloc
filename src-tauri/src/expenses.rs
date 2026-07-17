use crate::db::DbState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct Expense {
    pub id: Option<i64>,
    pub description: String,
    pub amount: f64,
    pub date: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Reminder {
    pub id: Option<i64>,
    pub account_id: i64,
    pub note: String,
    pub due_date: String,
    pub status: String,
}

#[tauri::command]
pub fn create_expense(state: State<'_, DbState>, payload: Expense) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO expenses (description, amount, date) VALUES (?1, ?2, ?3)",
        params![payload.description, payload.amount, payload.date],
    )
    .map_err(|e| e.to_string())?;

    Ok("Expense logged successfully".to_string())
}

#[tauri::command]
pub fn get_expenses(state: State<'_, DbState>) -> Result<Vec<Expense>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, description, amount, date FROM expenses ORDER BY date DESC")
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Expense {
                id: Some(row.get(0)?),
                description: row.get(1)?,
                amount: row.get(2)?,
                date: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for item in iter {
        if let Ok(val) = item {
            list.push(val);
        }
    }
    Ok(list)
}

#[tauri::command]
pub fn create_reminder(state: State<'_, DbState>, payload: Reminder) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO reminders (account_id, note, due_date, status) VALUES (?1, ?2, ?3, ?4)",
        params![payload.account_id, payload.note, payload.due_date, payload.status],
    )
    .map_err(|e| e.to_string())?;

    Ok("Reminder created successfully".to_string())
}

#[tauri::command]
pub fn get_reminders(state: State<'_, DbState>) -> Result<Vec<Reminder>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, account_id, note, due_date, status FROM reminders ORDER BY due_date ASC")
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Reminder {
                id: Some(row.get(0)?),
                account_id: row.get(1)?,
                note: row.get(2)?,
                due_date: row.get(3)?,
                status: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for item in iter {
        if let Ok(val) = item {
            list.push(val);
        }
    }
    Ok(list)
}
