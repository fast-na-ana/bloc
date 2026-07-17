use crate::db::DbState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct Payment {
    pub id: Option<i64>,
    pub account_id: i64,
    pub payment_type: String, // 'receipt' or 'payment'
    pub amount: f64,
    pub tid: String, // Transaction ID
    pub document_date: String,
    pub bank_cash_name: String,
}

#[tauri::command]
pub fn create_payment(state: State<'_, DbState>, payload: Payment) -> Result<String, String> {
    let mut conn = state.0.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Duplicate TID check
    let exists = {
        let mut stmt = tx.prepare("SELECT id FROM payments WHERE tid = ?1 LIMIT 1").map_err(|e| e.to_string())?;
        stmt.exists(params![payload.tid]).map_err(|e| e.to_string())?
    };
    
    if exists {
        return Err(format!("Duplicate TID Protection: A payment with Transaction ID '{}' already exists.", payload.tid));
    }

    tx.execute(
        "INSERT INTO payments (account_id, type, amount, tid, document_date, bank_cash_name) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![payload.account_id, payload.payment_type, payload.amount, payload.tid, payload.document_date, payload.bank_cash_name],
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok("Payment recorded successfully".to_string())
}

#[tauri::command]
pub fn get_payments(state: State<'_, DbState>) -> Result<Vec<Payment>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, account_id, type, amount, tid, document_date, bank_cash_name FROM payments ORDER BY document_date DESC")
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Payment {
                id: Some(row.get(0)?),
                account_id: row.get(1)?,
                payment_type: row.get(2)?,
                amount: row.get(3)?,
                tid: row.get(4)?,
                document_date: row.get(5)?,
                bank_cash_name: row.get(6)?,
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
