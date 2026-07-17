use crate::db::DbState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct LedgerEntry {
    pub date: String,
    pub description: String,
    pub debit: f64,
    pub credit: f64,
    pub balance: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemLedgerEntry {
    pub date: String,
    pub description: String,
    pub quantity_in: f64,
    pub quantity_out: f64,
    pub balance: f64,
}

#[tauri::command]
pub fn get_account_ledger(state: State<'_, DbState>, account_id: i64, role: String) -> Result<Vec<LedgerEntry>, String> {
    let conn = state.0.lock().unwrap();
    
    // role: "customer" or "supplier"
    // For customers: 
    // Sales (invoices) increase balance (Debit)
    // Payments received decrease balance (Credit)
    
    // For suppliers:
    // Purchases increase balance (Credit)
    // Payments sent decrease balance (Debit)

    let mut stmt = conn.prepare(
        "
        SELECT date, type as description, total as amount, 'invoice' as source 
        FROM invoices WHERE account_id = ?1
        UNION ALL
        SELECT document_date as date, type as description, amount, 'payment' as source 
        FROM payments WHERE account_id = ?1
        ORDER BY date ASC
        "
    ).map_err(|e| e.to_string())?;

    let iter = stmt.query_map(params![account_id], |row| {
        let date: String = row.get(0)?;
        let desc: String = row.get(1)?;
        let amount: f64 = row.get(2)?;
        let source: String = row.get(3)?;
        Ok((date, desc, amount, source))
    }).map_err(|e| e.to_string())?;

    let mut ledger = Vec::new();
    let mut running_balance = 0.0;

    for item in iter {
        if let Ok((date, desc, amount, source)) = item {
            let mut debit = 0.0;
            let mut credit = 0.0;

            if role == "customer" {
                if source == "invoice" && desc == "sale" {
                    debit = amount;
                    running_balance += amount;
                } else if source == "payment" && desc == "receipt" {
                    credit = amount;
                    running_balance -= amount;
                } else if source == "invoice" && desc == "sale_return" {
                    credit = amount;
                    running_balance -= amount;
                }
            } else if role == "supplier" {
                if source == "invoice" && desc == "purchase" {
                    credit = amount;
                    running_balance += amount;
                } else if source == "payment" && desc == "payment" {
                    debit = amount;
                    running_balance -= amount;
                } else if source == "invoice" && desc == "purchase_return" {
                    debit = amount;
                    running_balance -= amount;
                }
            }

            if debit > 0.0 || credit > 0.0 {
                ledger.push(LedgerEntry {
                    date,
                    description: desc,
                    debit,
                    credit,
                    balance: running_balance,
                });
            }
        }
    }

    Ok(ledger)
}

#[tauri::command]
pub fn get_item_ledger(state: State<'_, DbState>, inventory_id: i64) -> Result<Vec<ItemLedgerEntry>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "
        SELECT i.date, i.type, it.quantity 
        FROM invoices i
        JOIN invoice_items it ON i.id = it.invoice_id
        WHERE it.inventory_id = ?1
        ORDER BY i.date ASC
        "
    ).map_err(|e| e.to_string())?;

    let iter = stmt.query_map(params![inventory_id], |row| {
        let date: String = row.get(0)?;
        let inv_type: String = row.get(1)?;
        let qty: f64 = row.get(2)?;
        Ok((date, inv_type, qty))
    }).map_err(|e| e.to_string())?;

    let mut ledger = Vec::new();
    let mut running_balance = 0.0;

    for item in iter {
        if let Ok((date, inv_type, qty)) = item {
            let mut q_in = 0.0;
            let mut q_out = 0.0;

            if inv_type == "purchase" || inv_type == "sale_return" {
                q_in = qty;
                running_balance += qty;
            } else if inv_type == "sale" || inv_type == "purchase_return" {
                q_out = qty;
                running_balance -= qty;
            }

            ledger.push(ItemLedgerEntry {
                date,
                description: inv_type,
                quantity_in: q_in,
                quantity_out: q_out,
                balance: running_balance,
            });
        }
    }

    Ok(ledger)
}
