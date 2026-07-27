use crate::db::DbState;
use crate::journal::{post_journal_entry, JournalLine};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct Payment {
    pub id: Option<i64>,
    pub account_id: i64,
    pub payment_type: String, // 'receipt' or 'payment'
    pub amount: f64,          // display units (rupees)
    pub tid: String,          // Transaction ID
    pub document_date: String,
    pub settlement_account_code: String, // e.g. '1000' for Cash/Bank
    pub user_id: Option<i64>,
}

#[tauri::command]
pub fn create_payment(state: State<'_, DbState>, payload: Payment) -> Result<String, String> {
    let mut conn = state.0.lock().unwrap();
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    // Duplicate TID check
    let exists = {
        let mut stmt = tx
            .prepare("SELECT id FROM payments WHERE tid = ?1 LIMIT 1")
            .map_err(|e| e.to_string())?;
        stmt.exists(params![payload.tid]).map_err(|e| e.to_string())?
    };

    if exists {
        return Err(format!(
            "Duplicate TID Protection: A payment with Transaction ID '{}' already exists.",
            payload.tid
        ));
    }

    let amount_paisa = (payload.amount * 100.0).round() as i64;
    let created_by = payload.user_id.unwrap_or(0);

    tx.execute(
        "INSERT INTO payments (account_id, type, amount, tid, document_date, settlement_account_code)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            payload.account_id,
            payload.payment_type,
            amount_paisa,
            payload.tid,
            payload.document_date,
            payload.settlement_account_code
        ],
    )
    .map_err(|e| e.to_string())?;

    let payment_id = tx.last_insert_rowid();

    // Post double-entry journal
    // Receipt: customer pays us → Dr Cash/Bank (settlement), Cr AR
    // Payment: we pay supplier → Dr AP, Cr Cash/Bank (settlement)
    if created_by > 0 {
        let settle = payload.settlement_account_code.clone();
        let lines: Vec<JournalLine> = match payload.payment_type.as_str() {
            "receipt" => vec![
                JournalLine {
                    account_code: settle,
                    party_id: None,
                    debit: amount_paisa,
                    credit: 0,
                },
                JournalLine {
                    account_code: "1100".into(),
                    party_id: Some(payload.account_id),
                    debit: 0,
                    credit: amount_paisa,
                },
            ],
            "payment" => vec![
                JournalLine {
                    account_code: "2000".into(),
                    party_id: Some(payload.account_id),
                    debit: amount_paisa,
                    credit: 0,
                },
                JournalLine {
                    account_code: settle,
                    party_id: None,
                    debit: 0,
                    credit: amount_paisa,
                },
            ],
            other => {
                return Err(format!("Unknown payment type: {}", other));
            }
        };

        post_journal_entry(
            &tx,
            &payload.document_date,
            &format!("{} payment #{}", payload.payment_type, payment_id),
            "payment",
            Some(payment_id),
            None,
            created_by,
            &lines,
        )?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok("Payment recorded successfully".to_string())
}

#[tauri::command]
pub fn get_payments(state: State<'_, DbState>) -> Result<Vec<Payment>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, account_id, type, amount, tid, document_date, settlement_account_code
             FROM payments ORDER BY document_date DESC",
        )
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Payment {
                id: Some(row.get(0)?),
                account_id: row.get(1)?,
                payment_type: row.get(2)?,
                amount: row.get::<_, i64>(3)? as f64 / 100.0,
                tid: row.get(4)?,
                document_date: row.get(5)?,
                settlement_account_code: row.get(6)?,
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
