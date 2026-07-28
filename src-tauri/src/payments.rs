use crate::db::DbState;
use crate::journal::{post_journal_entry, JournalLine};
use rusqlite::{params, Connection};
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

pub fn create_payment_inner(conn: &mut Connection, payload: Payment) -> Result<String, String> {
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
    // Receipt: customer pays us → Dr Cash/Bank (settlement), Cr AR (or Cr 2200 if overpayment)
    // Payment: we pay supplier → Dr AP, Cr Cash/Bank (settlement)
    if created_by > 0 {
        let settle = payload.settlement_account_code.clone();
        let lines: Vec<JournalLine> = match payload.payment_type.as_str() {
            "receipt" => {
                // FIFO: collect open sale invoices before any inserts to release the borrow
                let open_invoices: Vec<(i64, i64)> = {
                    let mut stmt = tx
                        .prepare(
                            "SELECT i.id, i.total - COALESCE(SUM(pa.amount), 0) AS open_amount
                             FROM invoices i
                             LEFT JOIN payment_allocations pa ON pa.invoice_id = i.id
                             WHERE i.account_id = ?1 AND i.type = 'sale' AND i.voided_at IS NULL
                             GROUP BY i.id
                             HAVING open_amount > 0
                             ORDER BY i.date ASC, i.id ASC",
                        )
                        .map_err(|e| e.to_string())?;
                    let result: Vec<(i64, i64)> =
                        stmt.query_map(params![payload.account_id], |r| Ok((r.get(0)?, r.get(1)?)))
                        .map_err(|e| e.to_string())?
                        .filter_map(|r| r.ok())
                        .collect();
                    result
                };

                let mut remaining = amount_paisa;
                for (inv_id, open_amount) in &open_invoices {
                    if remaining <= 0 {
                        break;
                    }
                    let alloc = remaining.min(*open_amount);
                    tx.execute(
                        "INSERT INTO payment_allocations (payment_id, invoice_id, amount, allocation_source) \
                         VALUES (?1, ?2, ?3, 'payment')",
                        params![payment_id, inv_id, alloc],
                    )
                    .map_err(|e| e.to_string())?;
                    remaining -= alloc;
                }

                let overpayment = remaining; // > 0 when payment exceeds total open AR
                let ar_credit = amount_paisa - overpayment;
                let mut v = vec![JournalLine {
                    account_code: settle,
                    party_id: None,
                    debit: amount_paisa,
                    credit: 0,
                }];
                if ar_credit > 0 {
                    v.push(JournalLine {
                        account_code: "1100".into(),
                        party_id: Some(payload.account_id),
                        debit: 0,
                        credit: ar_credit,
                    });
                }
                if overpayment > 0 {
                    v.push(JournalLine {
                        account_code: "2200".into(),
                        party_id: Some(payload.account_id),
                        debit: 0,
                        credit: overpayment,
                    });
                }
                v
            }
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
pub fn create_payment(state: State<'_, DbState>, payload: Payment) -> Result<String, String> {
    let mut conn = state.0.lock().unwrap();
    create_payment_inner(&mut conn, payload)
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
