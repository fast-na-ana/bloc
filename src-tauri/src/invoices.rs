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
    pub settlement_account_code: Option<String>, // None = credit sale (AR); Some = cash sale
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
            "INSERT INTO invoices (type, account_id, date, total, discount, settlement_account_code)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                payload.invoice_type,
                payload.account_id,
                payload.date,
                total_paisa,
                discount_paisa,
                payload.settlement_account_code
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.last_insert_rowid()
    };

    let mut cogs_paisa: i64 = 0;
    let mut return_cogs_paisa: i64 = 0;

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

                    let cost_at_sale_total =
                        (val as f64 * item.quantity / qty as f64).round() as i64;
                    let new_val = val - cost_at_sale_total;
                    if new_val < 0 {
                        return Err(format!(
                            "Stock value would go negative for item {}: check data integrity",
                            item.inventory_id
                        ));
                    }
                    cogs_paisa += cost_at_sale_total;
                    (Some(cost_at_sale_total), -(item.quantity.round() as i64), -cost_at_sale_total)
                }
                "purchase" => {
                    // Update WAC accumulators: add qty and value
                    (None, item.quantity.round() as i64, line_total_paisa)
                }
                "sale_return" => {
                    // Restore inventory at original cost from the most recent matching sale line.
                    // Partial returns use proportional reversal: ROUND(orig_total * ret_qty / orig_qty).
                    let (orig_cost_total, orig_qty): (i64, f64) = tx
                        .query_row(
                            "SELECT COALESCE(ii.cost_at_sale, 0), ii.quantity
                             FROM invoice_items ii JOIN invoices i ON i.id = ii.invoice_id
                             WHERE ii.inventory_id = ?1 AND i.type = 'sale' AND i.voided_at IS NULL
                             ORDER BY ii.id DESC LIMIT 1",
                            params![item.inventory_id],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .unwrap_or((0, item.quantity));
                    let cogs_reversal = if orig_qty > 0.0 {
                        (orig_cost_total as f64 * item.quantity / orig_qty).round() as i64
                    } else {
                        0
                    };
                    return_cogs_paisa += cogs_reversal;
                    (Some(cogs_reversal), item.quantity.round() as i64, cogs_reversal)
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
                    let val_reduction =
                        (val as f64 * item.quantity / qty as f64).round() as i64;
                    if val - val_reduction < 0 {
                        return Err(format!(
                            "Purchase return would make inventory value negative for item {} — post an adjustment instead",
                            item.inventory_id
                        ));
                    }
                    (None, -ret_qty, -val_reduction)
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
            let mut v = match &payload.settlement_account_code {
                None => {
                    // Credit sale: Dr AR / Cr Revenue
                    vec![
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
                    ]
                }
                Some(settle) => {
                    // Cash sale: round total to nearest rupee; route any difference to 9000
                    let rounded = (total_paisa + 50) / 100 * 100;
                    let diff = rounded - total_paisa; // >0 = round up, <0 = round down
                    let mut cash_lines = vec![
                        JournalLine {
                            account_code: settle.clone(),
                            party_id: None,
                            debit: rounded,
                            credit: 0,
                        },
                        JournalLine {
                            account_code: "4000".into(),
                            party_id: None,
                            debit: 0,
                            credit: total_paisa,
                        },
                    ];
                    if diff > 0 {
                        cash_lines.push(JournalLine {
                            account_code: "9000".into(),
                            party_id: None,
                            debit: 0,
                            credit: diff,
                        });
                    } else if diff < 0 {
                        cash_lines.push(JournalLine {
                            account_code: "9000".into(),
                            party_id: None,
                            debit: -diff,
                            credit: 0,
                        });
                    }
                    cash_lines
                }
            };
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
        "sale_return" => {
            let mut v = vec![
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
            ];
            if return_cogs_paisa > 0 {
                v.push(JournalLine {
                    account_code: "1200".into(),
                    party_id: None,
                    debit: return_cogs_paisa,
                    credit: 0,
                });
                v.push(JournalLine {
                    account_code: "5000".into(),
                    party_id: None,
                    debit: 0,
                    credit: return_cogs_paisa,
                });
            }
            v
        }
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

pub fn void_invoice_inner(
    conn: &mut Connection,
    invoice_id: i64,
    voided_by: i64,
    void_reason: String,
) -> Result<String, String> {
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    let (invoice_type, already_voided): (String, bool) = tx
        .query_row(
            "SELECT type, voided_at IS NOT NULL FROM invoices WHERE id = ?1",
            params![invoice_id],
            |r| Ok((r.get(0)?, r.get::<_, bool>(1)?)),
        )
        .map_err(|e| format!("Invoice {} not found: {}", invoice_id, e))?;

    if already_voided {
        return Err(format!("Invoice {} is already voided", invoice_id));
    }

    tx.execute(
        "UPDATE invoices SET voided_at = CURRENT_TIMESTAMP, voided_by = ?1, void_reason = ?2 WHERE id = ?3",
        params![voided_by, void_reason, invoice_id],
    )
    .map_err(|e| e.to_string())?;

    // Find the original (non-reversing) journal entry for this invoice
    let orig_je_id: i64 = tx
        .query_row(
            "SELECT id FROM journal_entries
             WHERE reference_type = 'invoice' AND reference_id = ?1
               AND reverses_journal_entry_id IS NULL
             ORDER BY id ASC LIMIT 1",
            params![invoice_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("No journal entry for invoice {}: {}", invoice_id, e))?;

    let orig_date: String = tx
        .query_row(
            "SELECT entry_date FROM journal_entries WHERE id = ?1",
            params![orig_je_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Collect original ledger lines, then release the borrow
    let reversal_lines: Vec<JournalLine> = {
        let mut stmt = tx
            .prepare(
                "SELECT account_code, party_id, debit, credit FROM ledger_entries
                 WHERE journal_entry_id = ?1 ORDER BY id",
            )
            .map_err(|e| e.to_string())?;
        let result: Vec<JournalLine> = stmt
            .query_map(params![orig_je_id], |r| {
                Ok(JournalLine {
                    account_code: r.get(0)?,
                    party_id: r.get(1)?,
                    debit: r.get(3)?,  // swap: original credit → reversal debit
                    credit: r.get(2)?, // swap: original debit → reversal credit
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    post_journal_entry(
        &tx,
        &orig_date,
        &format!("Void of invoice #{}", invoice_id),
        "void",
        Some(invoice_id),
        Some(orig_je_id),
        voided_by,
        &reversal_lines,
    )?;

    // Reverse inventory changes
    let items: Vec<(i64, f64, Option<i64>, i64)> = {
        let mut stmt = tx
            .prepare(
                "SELECT inventory_id, quantity, cost_at_sale, total FROM invoice_items WHERE invoice_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let result: Vec<(i64, f64, Option<i64>, i64)> = stmt
            .query_map(params![invoice_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    for (inv_id, quantity, cost_at_sale, line_total) in &items {
        let (qty_delta, val_delta): (i64, i64) = match invoice_type.as_str() {
            "sale" => (quantity.round() as i64, cost_at_sale.unwrap_or(0)),
            "purchase" => (-quantity.round() as i64, -line_total),
            "sale_return" => (-quantity.round() as i64, -(cost_at_sale.unwrap_or(0))),
            "purchase_return" => (quantity.round() as i64, cost_at_sale.unwrap_or(*line_total)),
            _ => (0, 0),
        };

        if qty_delta != 0 || val_delta != 0 {
            tx.execute(
                "UPDATE inventory SET qty_on_hand = qty_on_hand + ?1, total_value_paisa = total_value_paisa + ?2 WHERE id = ?3",
                params![qty_delta, val_delta, inv_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(format!("Invoice #{} voided successfully", invoice_id))
}

#[tauri::command]
pub fn void_invoice(
    state: State<'_, DbState>,
    invoice_id: i64,
    voided_by: i64,
    void_reason: String,
) -> Result<String, String> {
    let mut conn = state.0.lock().unwrap();
    void_invoice_inner(&mut conn, invoice_id, voided_by, void_reason)
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
