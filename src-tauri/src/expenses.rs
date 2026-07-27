use crate::db::DbState;
use crate::journal::{post_journal_entry, JournalLine};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct Expense {
    pub id: Option<i64>,
    pub description: String,
    pub amount: f64, // display units (rupees)
    pub date: String,
    pub user_id: Option<i64>,
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
    let mut conn = state.0.lock().unwrap();
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    let amount_paisa = (payload.amount * 100.0).round() as i64;
    let created_by = payload.user_id.unwrap_or(0);

    tx.execute(
        "INSERT INTO expenses (description, amount, date) VALUES (?1, ?2, ?3)",
        params![payload.description, amount_paisa, payload.date],
    )
    .map_err(|e| e.to_string())?;

    let expense_id = tx.last_insert_rowid();

    // Post double-entry journal: Dr Operating Expenses / Cr Cash/Bank
    if created_by > 0 {
        let lines = vec![
            JournalLine {
                account_code: "6000".into(),
                party_id: None,
                debit: amount_paisa,
                credit: 0,
            },
            JournalLine {
                account_code: "1000".into(),
                party_id: None,
                debit: 0,
                credit: amount_paisa,
            },
        ];

        post_journal_entry(
            &tx,
            &payload.date,
            &format!("Expense: {}", payload.description),
            "expense",
            Some(expense_id),
            None,
            created_by,
            &lines,
        )?;
    }

    tx.commit().map_err(|e| e.to_string())?;
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
                amount: row.get::<_, i64>(2)? as f64 / 100.0,
                date: row.get(3)?,
                user_id: None,
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
        .prepare(
            "SELECT id, account_id, note, due_date, status FROM reminders ORDER BY due_date ASC",
        )
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
