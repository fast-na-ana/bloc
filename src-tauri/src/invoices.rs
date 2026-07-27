use crate::db::DbState;
use crate::journal::{post_journal_entry, JournalLine};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub account_type: String, // mapped from 'type'
    pub credit_limit: f64,    // display units (rupees)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InventoryItem {
    pub id: i64,
    pub name: String,
    pub stock_level: f64, // mirrors qty_on_hand for frontend compat
    pub cost_price: f64,  // display units
    pub sale_price: f64,  // display units
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InvoiceItemData {
    pub inventory_id: i64,
    pub quantity: f64,
    pub rate: f64,     // display units
    pub discount: f64, // display units
    pub total: f64,    // display units
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InvoiceData {
    pub invoice_type: String, // 'sale', 'purchase', 'sale_return', 'purchase_return'
    pub account_id: i64,
    pub date: String,
    pub total: f64,    // display units
    pub discount: f64, // display units
    pub items: Vec<InvoiceItemData>,
    pub user_id: Option<i64>,
}

#[tauri::command]
pub fn get_accounts(state: State<'_, DbState>) -> Result<Vec<Account>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, type, credit_limit FROM accounts")
        .map_err(|e| e.to_string())?;
    let iter = stmt
        .query_map([], |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                account_type: row.get(2)?,
                credit_limit: row.get::<_, i64>(3)? as f64 / 100.0,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in iter {
        if let Ok(v) = r {
            out.push(v);
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn get_inventory(state: State<'_, DbState>) -> Result<Vec<InventoryItem>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, qty_on_hand, cost_price, sale_price FROM inventory")
        .map_err(|e| e.to_string())?;
    let iter = stmt
        .query_map([], |row| {
            Ok(InventoryItem {
                id: row.get(0)?,
                name: row.get(1)?,
                stock_level: row.get::<_, i64>(2)? as f64,
                cost_price: row.get::<_, i64>(3)? as f64 / 100.0,
                sale_price: row.get::<_, i64>(4)? as f64 / 100.0,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in iter {
        if let Ok(v) = r {
            out.push(v);
        }
    }
    Ok(out)
}

pub fn create_invoice_inner(conn: &mut Connection, payload: InvoiceData) -> Result<String, String> {
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    let total_paisa = (payload.total * 100.0).round() as i64;
    let discount_paisa = (payload.discount * 100.0).round() as i64;
    let created_by = payload.user_id.unwrap_or(0);

    // Auto-merge: same date + account + type
    let existing_id: Option<i64> = {
        let mut stmt = tx
            .prepare(
                "SELECT id FROM invoices WHERE account_id=?1 AND date=?2 AND type=?3 LIMIT 1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query(params![payload.account_id, payload.date, payload.invoice_type])
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Some(row.get(0).map_err(|e| e.to_string())?)
        } else {
            None
        }
    };

    let invoice_id = if let Some(id) = existing_id {
        tx.execute(
            "UPDATE invoices SET total = total + ?1, discount = discount + ?2 WHERE id = ?3",
            params![total_paisa, discount_paisa, id],
        )
        .map_err(|e| e.to_string())?;
        id
    } else {
        tx.execute(
            "INSERT INTO invoices (type, account_id, date, total, discount) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                payload.invoice_type,
                payload.account_id,
                payload.date,
                total_paisa,
                discount_paisa
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.last_insert_rowid()
    };

    let mut cogs_paisa: i64 = 0;

    for item in &payload.items {
        let rate_paisa = (item.rate * 100.0).round() as i64;
        let item_discount_paisa = (item.discount * 100.0).round() as i64;
        let line_total_paisa = (item.total * 100.0).round() as i64;

        // WAC LIMITATION: a purchase entered after the fact updates the running average
        // going forward only. Sales already committed after that purchase date (but before
        // entry) retain their original cost_at_sale snapshot. This is correct under
        // immutability — do not treat it as a bug.

        // Compute cost_at_sale and inventory deltas based on invoice type.
        // We compute everything before the INSERT so sale lines have a single
        // cost snapshot used consistently for both the invoice_items record
        // and the inventory update.
        let (cost_at_sale, qty_delta, val_delta): (Option<i64>, i64, i64) =
            match payload.invoice_type.as_str() {
                "sale" => {
                    // Snapshot current WAC; enforce stock availability
                    let (qty, val): (i64, i64) = tx
                        .query_row(
                            "SELECT qty_on_hand, total_value_paisa FROM inventory WHERE id = ?1",
                            params![item.inventory_id],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .map_err(|e| e.to_string())?;

                    if qty <= 0 {
                        return Err(format!(
                            "Insufficient stock for item {}: qty_on_hand is {}",
                            item.inventory_id, qty
                        ));
                    }
                    let needed = item.quantity.ceil() as i64;
                    if needed > qty {
                        return Err(format!(
                            "Insufficient stock for item {}: need {} but qty_on_hand is {}",
                            item.inventory_id, needed, qty
                        ));
                    }

                    let avg_paisa = (val as f64 / qty as f64).round() as i64;
                    let cost_reduction = (avg_paisa as f64 * item.quantity).round() as i64;
                    let new_val = val - cost_reduction;
                    if new_val < 0 {
                        return Err(format!(
                            "Stock value would go negative for item {}: check data integrity",
                            item.inventory_id
                        ));
                    }
                    cogs_paisa += cost_reduction;
                    (Some(avg_paisa), -(item.quantity.round() as i64), -cost_reduction)
                }
                "purchase" => {
                    // Update WAC accumulators: add qty and value
                    (None, item.quantity.round() as i64, line_total_paisa)
                }
                "sale_return" => {
                    // Restore quantity; value restored at current WAC (best approximation)
                    let (qty, val): (i64, i64) = tx
                        .query_row(
                            "SELECT qty_on_hand, total_value_paisa FROM inventory WHERE id = ?1",
                            params![item.inventory_id],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .map_err(|e| e.to_string())?;
                    let avg =
                        if qty > 0 { (val as f64 / qty as f64).round() as i64 } else { 0 };
                    let restored_val = (avg as f64 * item.quantity).round() as i64;
                    (None, item.quantity.round() as i64, restored_val)
                }
                "purchase_return" => {
                    let (qty, val): (i64, i64) = tx
                        .query_row(
                            "SELECT qty_on_hand, total_value_paisa FROM inventory WHERE id = ?1",
                            params![item.inventory_id],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .map_err(|e| e.to_string())?;
                    if qty == 0 {
                        return Err(format!(
                            "Cannot process purchase return for item {}: qty_on_hand is zero",
                            item.inventory_id
                        ));
                    }
                    let ret_qty = item.quantity.round() as i64;
                    if ret_qty > qty {
                        return Err(format!(
                            "Purchase return qty {} exceeds qty_on_hand {} for item {}",
                            ret_qty, qty, item.inventory_id
                        ));
                    }
                    let avg = (val as f64 / qty as f64).round() as i64;
                    let val_reduction = (avg as f64 * item.quantity).round() as i64;
                    if val - val_reduction < 0 {
                        return Err(format!(
                            "Purchase return would make inventory value negative for item {} — post an adjustment instead",
                            item.inventory_id
                        ));
                    }
                    cogs_paisa += val_reduction;
                    (Some(avg), -ret_qty, -val_reduction)
                }
                _ => (None, 0, 0),
            };

        tx.execute(
            "INSERT INTO invoice_items (invoice_id, inventory_id, quantity, rate, discount, total, cost_at_sale)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                invoice_id,
                item.inventory_id,
                item.quantity,
                rate_paisa,
                item_discount_paisa,
                line_total_paisa,
                cost_at_sale
            ],
        )
        .map_err(|e| e.to_string())?;

        tx.execute(
            "UPDATE inventory SET qty_on_hand = qty_on_hand + ?1, total_value_paisa = total_value_paisa + ?2 WHERE id = ?3",
            params![qty_delta, val_delta, item.inventory_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Post double-entry journal.
    // Zero-value COGS lines are excluded because the ledger_entries CHECK constraint
    // requires (debit > 0 AND credit = 0) OR (credit > 0 AND debit = 0).
    // purchase, sale_return, and purchase_return have no COGS lines in their fixed sets.
    let lines: Vec<JournalLine> = match payload.invoice_type.as_str() {
        "sale" => {
            let mut v = vec![
                JournalLine {
                    account_code: "1100".into(),
                    party_id: Some(payload.account_id),
                    debit: total_paisa,
                    credit: 0,
                },
                JournalLine {
                    account_code: "4000".into(),
                    party_id: None,
                    debit: 0,
                    credit: total_paisa,
                },
            ];
            if cogs_paisa > 0 {
                v.push(JournalLine {
                    account_code: "5000".into(),
                    party_id: None,
                    debit: cogs_paisa,
                    credit: 0,
                });
                v.push(JournalLine {
                    account_code: "1200".into(),
                    party_id: None,
                    debit: 0,
                    credit: cogs_paisa,
                });
            }
            v
        }
        "purchase" => vec![
            JournalLine {
                account_code: "1200".into(),
                party_id: None,
                debit: total_paisa,
                credit: 0,
            },
            JournalLine {
                account_code: "2000".into(),
                party_id: Some(payload.account_id),
                debit: 0,
                credit: total_paisa,
            },
        ],
        "sale_return" => vec![
            JournalLine {
                account_code: "4100".into(),
                party_id: None,
                debit: total_paisa,
                credit: 0,
            },
            JournalLine {
                account_code: "1100".into(),
                party_id: Some(payload.account_id),
                debit: 0,
                credit: total_paisa,
            },
        ],
        "purchase_return" => vec![
            JournalLine {
                account_code: "2000".into(),
                party_id: Some(payload.account_id),
                debit: total_paisa,
                credit: 0,
            },
            JournalLine {
                account_code: "5100".into(),
                party_id: None,
                debit: 0,
                credit: total_paisa,
            },
        ],
        _ => return Err(format!("Unknown invoice type: {}", payload.invoice_type)),
    };

    // Only post journal if we have a real user (skip for legacy data without user context)
    if created_by > 0 {
        post_journal_entry(
            &tx,
            &payload.date,
            &format!("{} invoice #{}", payload.invoice_type, invoice_id),
            "invoice",
            Some(invoice_id),
            None,
            created_by,
            &lines,
        )?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(format!("Successfully created/merged invoice #{}", invoice_id))
}

#[tauri::command]
pub fn create_invoice(state: State<'_, DbState>, payload: InvoiceData) -> Result<String, String> {
    let mut conn = state.0.lock().unwrap();
    create_invoice_inner(&mut conn, payload)
}

#[tauri::command]
pub fn create_account(
    state: State<'_, DbState>,
    name: String,
    account_type: String,
    credit_limit: f64,
) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    let limit_paisa = (credit_limit * 100.0).round() as i64;
    conn.execute(
        "INSERT INTO accounts (name, type, credit_limit) VALUES (?1, ?2, ?3)",
        params![name, account_type, limit_paisa],
    )
    .map_err(|e| e.to_string())?;
    Ok("Account created successfully".to_string())
}

#[tauri::command]
pub fn create_inventory_item(
    state: State<'_, DbState>,
    name: String,
    stock_level: f64,
    cost_price: f64,
    sale_price: f64,
) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    let qty = stock_level.round() as i64;
    let cost_paisa = (cost_price * 100.0).round() as i64;
    let sale_paisa = (sale_price * 100.0).round() as i64;
    let total_val = (cost_paisa as f64 * stock_level).round() as i64;
    conn.execute(
        "INSERT INTO inventory (name, qty_on_hand, total_value_paisa, cost_price, sale_price) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, qty, total_val, cost_paisa, sale_paisa],
    )
    .map_err(|e| e.to_string())?;
    Ok("Inventory item created successfully".to_string())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaybackMetrics {
    pub average_days: f64,
    pub late_payments: i32,
}

#[tauri::command]
pub fn update_credit_limit(
    state: State<'_, DbState>,
    account_id: i64,
    new_limit: f64,
    current_role: String,
) -> Result<String, String> {
    if current_role != "admin" {
        return Err("Unauthorized: Only Admins can change credit limits.".to_string());
    }
    let conn = state.0.lock().unwrap();
    let limit_paisa = (new_limit * 100.0).round() as i64;
    conn.execute(
        "UPDATE accounts SET credit_limit = ?1 WHERE id = ?2",
        params![limit_paisa, account_id],
    )
    .map_err(|e| e.to_string())?;
    Ok("Credit limit updated successfully".to_string())
}

#[tauri::command]
pub fn analyze_payback_behavior(
    state: State<'_, DbState>,
    account_id: i64,
) -> Result<PaybackMetrics, String> {
    let conn = state.0.lock().unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT AVG(julianday(p.document_date) - julianday(i.date))
             FROM invoices i
             JOIN payments p ON p.account_id = i.account_id
             WHERE i.account_id = ?1
               AND i.type = 'sale'
               AND p.type = 'receipt'
               AND p.document_date >= i.date",
        )
        .map_err(|e| e.to_string())?;

    let avg_days: f64 = stmt
        .query_row(params![account_id], |r| Ok(r.get(0).unwrap_or(0.0)))
        .unwrap_or(0.0);

    let mut stmt2 = conn
        .prepare(
            "SELECT count(*) FROM reminders WHERE account_id = ?1 AND date(due_date) < date('now')",
        )
        .map_err(|e| e.to_string())?;

    let late_payments: i32 = stmt2
        .query_row(params![account_id], |r| Ok(r.get(0).unwrap_or(0)))
        .unwrap_or(0);

    Ok(PaybackMetrics {
        average_days: avg_days.round(),
        late_payments,
    })
}
