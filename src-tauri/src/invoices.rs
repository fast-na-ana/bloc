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
    pub item_type: String,
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
    pub original_invoice_id: Option<i64>,        // required for sale_return / purchase_return
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
        .prepare("SELECT id, name, qty_on_hand, cost_price, sale_price, item_type FROM inventory")
        .map_err(|e| e.to_string())?;
    let iter = stmt
        .query_map([], |row| {
            Ok(InventoryItem {
                id: row.get(0)?,
                name: row.get(1)?,
                stock_level: row.get::<_, i64>(2)? as f64,
                cost_price: row.get::<_, i64>(3)? as f64 / 100.0,
                sale_price: row.get::<_, i64>(4)? as f64 / 100.0,
                item_type: row.get(5)?,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct InvoiceSummary {
    pub id: i64,
    pub invoice_type: String, // 'sale', 'purchase', etc
    pub account_id: i64,
    pub account_name: String,
    pub date: String,
    pub total: f64,
    pub voided_at: Option<String>,
}

#[tauri::command]
pub fn get_invoices(state: State<'_, DbState>) -> Result<Vec<InvoiceSummary>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.type, i.account_id, a.name, i.date, i.total, i.voided_at
             FROM invoices i
             JOIN accounts a ON i.account_id = a.id
             ORDER BY i.date DESC, i.id DESC"
        )
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(InvoiceSummary {
                id: row.get(0)?,
                invoice_type: row.get(1)?,
                account_id: row.get(2)?,
                account_name: row.get(3)?,
                date: row.get(4)?,
                total: row.get::<_, i64>(5)? as f64 / 100.0,
                voided_at: row.get(6)?,
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

    // Auto-merge: same date + account + type — skipped for returns (each is a distinct document)
    let is_return = matches!(payload.invoice_type.as_str(), "sale_return" | "purchase_return");
    let existing_id: Option<i64> = if is_return {
        None
    } else {
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

    let (invoice_id, is_new) = if let Some(id) = existing_id {
        tx.execute(
            "UPDATE invoices SET total = total + ?1, discount = discount + ?2 WHERE id = ?3",
            params![total_paisa, discount_paisa, id],
        )
        .map_err(|e| e.to_string())?;
        (id, false)
    } else {
        tx.execute(
            "INSERT INTO invoices (type, account_id, date, total, discount, settlement_account_code, original_invoice_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                payload.invoice_type,
                payload.account_id,
                payload.date,
                total_paisa,
                discount_paisa,
                payload.settlement_account_code,
                payload.original_invoice_id
            ],
        )
        .map_err(|e| e.to_string())?;
        (tx.last_insert_rowid(), true)
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
                    let orig_inv_id = payload.original_invoice_id.ok_or_else(|| {
                        "sale_return requires original_invoice_id".to_string()
                    })?;

                    // Validate original invoice is a non-voided sale
                    let (orig_type, orig_voided): (String, bool) = tx
                        .query_row(
                            "SELECT type, voided_at IS NOT NULL FROM invoices WHERE id = ?1",
                            params![orig_inv_id],
                            |r| Ok((r.get(0)?, r.get::<_, bool>(1)?)),
                        )
                        .map_err(|_| format!("Original invoice {} not found", orig_inv_id))?;
                    if orig_voided {
                        return Err(format!(
                            "Cannot return against voided invoice {}",
                            orig_inv_id
                        ));
                    }
                    if orig_type != "sale" {
                        return Err(format!(
                            "Invoice {} is not a sale (type={})",
                            orig_inv_id, orig_type
                        ));
                    }

                    // Fetch cost_at_sale and quantity from the original sale line
                    let (orig_cost_total, orig_qty): (i64, f64) = tx
                        .query_row(
                            "SELECT COALESCE(cost_at_sale, 0), quantity
                             FROM invoice_items
                             WHERE invoice_id = ?1 AND inventory_id = ?2",
                            params![orig_inv_id, item.inventory_id],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .map_err(|_| format!(
                            "Item {} not found on invoice {}",
                            item.inventory_id, orig_inv_id
                        ))?;

                    // Ensure cumulative returned qty does not exceed original qty
                    let already_returned: f64 = tx
                        .query_row(
                            "SELECT COALESCE(SUM(ri.quantity), 0)
                             FROM invoices ret
                             JOIN invoice_items ri ON ri.invoice_id = ret.id
                             WHERE ret.original_invoice_id = ?1
                               AND ri.inventory_id = ?2
                               AND ret.voided_at IS NULL",
                            params![orig_inv_id, item.inventory_id],
                            |r| r.get(0),
                        )
                        .unwrap_or(0.0);
                    let remaining = orig_qty - already_returned;
                    if item.quantity > remaining + 0.001 {
                        return Err(format!(
                            "Return qty {:.4} exceeds remaining returnable {:.4} \
                             (original {:.4} already returned {:.4}) for item {} on invoice {}",
                            item.quantity, remaining, orig_qty, already_returned,
                            item.inventory_id, orig_inv_id
                        ));
                    }

                    // Proportional COGS reversal from the original sale line total
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
    let mut lines: Vec<JournalLine> = match payload.invoice_type.as_str() {
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
        // Advance auto-application: new credit sales only.
        // If the customer has unallocated prior overpayments (sitting in 2200), apply them
        // to this invoice now. Lines are appended to the invoice JE so void automatically
        // reverses them via mirror-flip. Writes payment_allocations with source='advance'
        // so FIFO sees the correct open amount for future receipts.
        if is_new && payload.invoice_type == "sale" && payload.settlement_account_code.is_none() {
            let unallocated: Vec<(i64, i64)> = {
                let mut stmt = tx
                    .prepare(
                        "SELECT p.id, p.amount - COALESCE(SUM(pa.amount), 0) AS unalloc
                         FROM payments p
                         LEFT JOIN payment_allocations pa ON pa.payment_id = p.id
                         WHERE p.account_id = ?1 AND p.type = 'receipt' AND p.voided_at IS NULL
                         GROUP BY p.id
                         HAVING unalloc > 0
                         ORDER BY p.document_date ASC, p.id ASC",
                    )
                    .map_err(|e| e.to_string())?;
                let result: Vec<(i64, i64)> = stmt
                    .query_map(params![payload.account_id], |r| Ok((r.get(0)?, r.get(1)?)))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();
                result
            };

            let mut advance_applied: i64 = 0;
            let mut remaining_open = total_paisa;
            for (pay_id, unalloc) in &unallocated {
                if remaining_open <= 0 {
                    break;
                }
                let apply = remaining_open.min(*unalloc);
                tx.execute(
                    "INSERT INTO payment_allocations (payment_id, invoice_id, amount, allocation_source) \
                     VALUES (?1, ?2, ?3, 'advance')",
                    params![pay_id, invoice_id, apply],
                )
                .map_err(|e| e.to_string())?;
                remaining_open -= apply;
                advance_applied += apply;
            }

            if advance_applied > 0 {
                lines.push(JournalLine {
                    account_code: "2200".into(),
                    party_id: Some(payload.account_id),
                    debit: advance_applied,
                    credit: 0,
                });
                lines.push(JournalLine {
                    account_code: "1100".into(),
                    party_id: Some(payload.account_id),
                    debit: 0,
                    credit: advance_applied,
                });
            }
        }

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

    let (invoice_type, _account_id, already_voided): (String, i64, bool) = tx
        .query_row(
            "SELECT type, account_id, voided_at IS NOT NULL FROM invoices WHERE id = ?1",
            params![invoice_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, bool>(2)?)),
        )
        .map_err(|e| format!("Invoice {} not found: {}", invoice_id, e))?;

    if already_voided {
        return Err(format!("Invoice {} is already voided", invoice_id));
    }

    // Block only on 'payment'-source allocations.
    // 'advance'-source allocations are cleaned up below; the JE reversal restores 2200.
    let alloc_sum: i64 = tx
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM payment_allocations \
             WHERE invoice_id = ?1 AND allocation_source = 'payment'",
            params![invoice_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if alloc_sum > 0 {
        return Err(format!(
            "Cannot void invoice {}: {} paisa of payments are allocated to it. \
             Void the payment(s) first.",
            invoice_id, alloc_sum
        ));
    }

    // Block void if any unvoided returns reference this invoice via original_invoice_id.
    // Applies to both sales and purchases; returns of returns don't exist.
    if matches!(invoice_type.as_str(), "sale" | "purchase") {
        let return_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM invoices \
                 WHERE original_invoice_id = ?1 AND voided_at IS NULL",
                params![invoice_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if return_count > 0 {
            return Err(format!(
                "Cannot void invoice {}: it has {} unvoided return(s) against it. \
                 Void the return invoice(s) first.",
                invoice_id, return_count
            ));
        }
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

    // Delete advance allocations. The JE reversal (below) already posts Dr 1100 / Cr 2200
    // to restore the advance, so keeping the allocation row would make the payment look
    // partially unallocated again when it is not.
    tx.execute(
        "DELETE FROM payment_allocations WHERE invoice_id = ?1 AND allocation_source = 'advance'",
        params![invoice_id],
    )
    .map_err(|e| e.to_string())?;

    // Collect original ledger lines, then release the borrow.
    // Use posted_ledger_entries so we only mirror-flip lines that are actually posted;
    // an unposted JE (shouldn't exist for invoices, but guards against data races) is ignored.
    let reversal_lines: Vec<JournalLine> = {
        let mut stmt = tx
            .prepare(
                "SELECT account_code, party_id, debit, credit FROM posted_ledger_entries
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
    item_type: String,
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
        "INSERT INTO inventory (name, item_type, qty_on_hand, total_value_paisa, cost_price, sale_price) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![name, item_type, qty, total_val, cost_paisa, sale_paisa],
    )
    .map_err(|e| e.to_string())?;
    Ok("Inventory item created successfully".to_string())
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

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountSummary {
    pub account_id: i64,
    pub outstanding: f64, // rupees; positive = owes us (AR) or we owe them (AP)
}

#[tauri::command]
pub fn get_account_summaries(state: State<'_, DbState>) -> Result<Vec<AccountSummary>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.type,
                COALESCE(
                    CASE
                        WHEN a.type = 'customer'
                            THEN SUM(le.debit) - SUM(le.credit)
                        WHEN a.type = 'supplier'
                            THEN SUM(le.credit) - SUM(le.debit)
                        ELSE 0
                    END,
                0) AS outstanding_paisa
             FROM accounts a
             LEFT JOIN posted_ledger_entries le
                ON le.party_id = a.id
                AND le.account_code = CASE
                    WHEN a.type = 'customer' THEN '1100'
                    WHEN a.type = 'supplier' THEN '2000'
                    ELSE NULL
                END
             GROUP BY a.id, a.type",
        )
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(AccountSummary {
                account_id: row.get(0)?,
                outstanding: row.get::<_, i64>(2)? as f64 / 100.0,
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
pub fn adjust_inventory_stock(
    state: State<'_, DbState>,
    inventory_id: i64,
    qty_change: f64,
    reason: String,
    user_id: i64,
) -> Result<String, String> {
    let mut conn = state.0.lock().unwrap();
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    let (_item_type, cost_price, qty_on_hand): (String, f64, i64) = tx.query_row(
        "SELECT item_type, cost_price / 100.0, qty_on_hand FROM inventory WHERE id = ?1",
        params![inventory_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    ).map_err(|e| e.to_string())?;

    let qty_change_int = qty_change.round() as i64;
    let new_qty = qty_on_hand + qty_change_int;
    
    let new_value_paisa = (new_qty as f64 * cost_price * 100.0).round() as i64;

    tx.execute(
        "UPDATE inventory SET qty_on_hand = ?1, total_value_paisa = ?2 WHERE id = ?3",
        params![new_qty, new_value_paisa, inventory_id],
    ).map_err(|e| e.to_string())?;

    let change_value_paisa = (qty_change_int.abs() as f64 * cost_price * 100.0).round() as i64;
    
    if change_value_paisa > 0 {
        let (dr, cr) = if qty_change_int > 0 {
            ("1200", "5000") // Add stock: Dr Inventory, Cr COGS
        } else {
            ("5000", "1200") // Remove stock: Dr COGS, Cr Inventory
        };

        let lines = vec![
            JournalLine {
                account_code: dr.into(),
                party_id: None,
                debit: change_value_paisa,
                credit: 0,
            },
            JournalLine {
                account_code: cr.into(),
                party_id: None,
                debit: 0,
                credit: change_value_paisa,
            },
        ];

        let now_str: String = tx.query_row("SELECT datetime('now', 'localtime')", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        post_journal_entry(&tx, &now_str, &reason, "adjustment", Some(inventory_id), None, user_id, &lines).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok("Inventory adjusted".to_string())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InvoiceDetailItem {
    pub id: i64,
    pub inventory_name: String,
    pub quantity: f64,
    pub rate: f64,
    pub discount: f64,
    pub total: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InvoiceDetail {
    pub id: i64,
    pub invoice_type: String,
    pub account_id: i64,
    pub account_name: String,
    pub date: String,
    pub subtotal: f64,
    pub global_discount: f64,
    pub grand_total: f64,
    pub is_cash: bool,
    pub voided_at: Option<String>,
    pub items: Vec<InvoiceDetailItem>,
}

#[tauri::command]
pub fn get_invoice_details(state: State<'_, DbState>, invoice_id: i64) -> Result<InvoiceDetail, String> {
    let conn = state.0.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT 
            i.id, i.type, i.account_id, a.name, i.date, 
            i.total, i.discount, i.voided_at
         FROM invoices i
         JOIN accounts a ON i.account_id = a.id
         WHERE i.id = ?1"
    ).map_err(|e| e.to_string())?;

    let mut rows = stmt.query(params![invoice_id]).map_err(|e| e.to_string())?;
    
    let row = rows.next().map_err(|e| e.to_string())?.ok_or_else(|| "Invoice not found".to_string())?;

    let id: i64 = row.get(0).unwrap_or(0);
    let invoice_type: String = row.get(1).unwrap_or_default();
    let account_id: i64 = row.get(2).unwrap_or(0);
    let account_name: String = row.get(3).unwrap_or_default();
    let date: String = row.get(4).unwrap_or_default();
    
    let grand_total: f64 = row.get::<_, i64>(5).unwrap_or(0) as f64 / 100.0;
    let global_discount: f64 = row.get::<_, i64>(6).unwrap_or(0) as f64 / 100.0;
    let voided_at: Option<String> = row.get(7).unwrap_or(None);
    
    let is_cash = account_name.to_lowercase().contains("cash");
    
    let mut item_stmt = conn.prepare(
        "SELECT 
            ii.id, inv.name, ii.quantity, ii.rate, ii.discount, ii.total
         FROM invoice_items ii
         JOIN inventory inv ON ii.inventory_id = inv.id
         WHERE ii.invoice_id = ?1"
    ).map_err(|e| e.to_string())?;
    
    let item_iter = item_stmt.query_map(params![invoice_id], |r| {
        Ok(InvoiceDetailItem {
            id: r.get(0)?,
            inventory_name: r.get(1)?,
            quantity: r.get::<_, f64>(2)?,
            rate: r.get::<_, i64>(3)? as f64 / 100.0,
            discount: r.get::<_, i64>(4)? as f64 / 100.0,
            total: r.get::<_, i64>(5)? as f64 / 100.0,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut items = Vec::new();
    let mut subtotal = 0.0;
    for item in item_iter {
        if let Ok(i) = item {
            subtotal += i.total;
            items.push(i);
        }
    }
    
    Ok(InvoiceDetail {
        id,
        invoice_type,
        account_id,
        account_name,
        date,
        subtotal,
        global_discount,
        grand_total,
        is_cash,
        voided_at,
        items,
    })
}
