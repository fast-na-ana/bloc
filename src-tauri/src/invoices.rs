use crate::db::DbState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub account_type: String, // mapped from 'type'
    pub credit_limit: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InventoryItem {
    pub id: i64,
    pub name: String,
    pub stock_level: f64,
    pub cost_price: f64,
    pub sale_price: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InvoiceItemData {
    pub inventory_id: i64,
    pub quantity: f64,
    pub rate: f64,
    pub discount: f64,
    pub total: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InvoiceData {
    pub invoice_type: String, // 'sale', 'purchase', 'sale_return', 'purchase_return'
    pub account_id: i64,
    pub date: String,
    pub total: f64,
    pub discount: f64,
    pub items: Vec<InvoiceItemData>,
}

#[tauri::command]
pub fn get_accounts(state: State<'_, DbState>) -> Result<Vec<Account>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, type, credit_limit FROM accounts")
        .map_err(|e| e.to_string())?;

    let account_iter = stmt
        .query_map([], |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                account_type: row.get(2)?,
                credit_limit: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut accounts = Vec::new();
    for acc in account_iter {
        if let Ok(account) = acc {
            accounts.push(account);
        }
    }

    Ok(accounts)
}

#[tauri::command]
pub fn get_inventory(state: State<'_, DbState>) -> Result<Vec<InventoryItem>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, stock_level, cost_price, sale_price FROM inventory")
        .map_err(|e| e.to_string())?;

    let item_iter = stmt
        .query_map([], |row| {
            Ok(InventoryItem {
                id: row.get(0)?,
                name: row.get(1)?,
                stock_level: row.get(2)?,
                cost_price: row.get(3)?,
                sale_price: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for item in item_iter {
        if let Ok(inventory) = item {
            items.push(inventory);
        }
    }

    Ok(items)
}

#[tauri::command]
pub fn create_invoice(state: State<'_, DbState>, payload: InvoiceData) -> Result<String, String> {
    let mut conn = state.0.lock().unwrap();
    
    // SQLite transaction
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // 1. Check for Auto-Merge: same date, same account, same type
    let mut existing_invoice_id: Option<i64> = None;
    
    {
        let mut stmt = tx.prepare(
            "SELECT id FROM invoices WHERE account_id = ?1 AND date = ?2 AND type = ?3 LIMIT 1"
        ).map_err(|e| e.to_string())?;
        
        let mut rows = stmt.query(params![payload.account_id, payload.date, payload.invoice_type])
            .map_err(|e| e.to_string())?;
            
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            existing_invoice_id = Some(row.get(0).map_err(|e| e.to_string())?);
        }
    }

    let invoice_id = if let Some(id) = existing_invoice_id {
        // Update existing invoice total and discount
        tx.execute(
            "UPDATE invoices SET total = total + ?1, discount = discount + ?2 WHERE id = ?3",
            params![payload.total, payload.discount, id],
        ).map_err(|e| e.to_string())?;
        id
    } else {
        // Create new invoice
        tx.execute(
            "INSERT INTO invoices (type, account_id, date, total, discount) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![payload.invoice_type, payload.account_id, payload.date, payload.total, payload.discount],
        ).map_err(|e| e.to_string())?;
        tx.last_insert_rowid()
    };

    // 2. Insert Items and Adjust Stock
    for item in payload.items {
        tx.execute(
            "INSERT INTO invoice_items (invoice_id, inventory_id, quantity, rate, discount, total) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![invoice_id, item.inventory_id, item.quantity, item.rate, item.discount, item.total],
        ).map_err(|e| e.to_string())?;

        // Stock calculation based on invoice type
        let stock_change = match payload.invoice_type.as_str() {
            "sale" => -item.quantity,
            "purchase" => item.quantity,
            "sale_return" => item.quantity,
            "purchase_return" => -item.quantity,
            _ => 0.0,
        };

        tx.execute(
            "UPDATE inventory SET stock_level = stock_level + ?1 WHERE id = ?2",
            params![stock_change, item.inventory_id],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(format!("Successfully created/merged invoice #{}", invoice_id))
}

#[tauri::command]
pub fn create_account(state: State<'_, DbState>, name: String, account_type: String, credit_limit: f64) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO accounts (name, type, credit_limit) VALUES (?1, ?2, ?3)",
        params![name, account_type, credit_limit],
    ).map_err(|e| e.to_string())?;
    
    Ok("Account created successfully".to_string())
}

#[tauri::command]
pub fn create_inventory_item(state: State<'_, DbState>, name: String, stock_level: f64, cost_price: f64, sale_price: f64) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO inventory (name, stock_level, cost_price, sale_price) VALUES (?1, ?2, ?3, ?4)",
        params![name, stock_level, cost_price, sale_price],
    ).map_err(|e| e.to_string())?;
    
    Ok("Inventory item created successfully".to_string())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaybackMetrics {
    pub average_days: f64,
    pub late_payments: i32,
}

#[tauri::command]
pub fn update_credit_limit(state: State<'_, DbState>, account_id: i64, new_limit: f64, password: String) -> Result<String, String> {
    // Temporary hardcoded password check for Phase 4
    if password != "admin" {
        return Err("Unauthorized: Invalid Admin/Manager password.".to_string());
    }

    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE accounts SET credit_limit = ?1 WHERE id = ?2",
        params![new_limit, account_id],
    ).map_err(|e| e.to_string())?;

    Ok("Credit limit updated successfully".to_string())
}

#[tauri::command]
pub fn analyze_payback_behavior(state: State<'_, DbState>, account_id: i64) -> Result<PaybackMetrics, String> {
    let conn = state.0.lock().unwrap();
    
    // Calculate average days to pay:
    // We approximate by comparing all sales invoice dates to all receipt dates for this account.
    // In a strict ledger, you match specific invoices, but an aggregate average of days is robust.
    // Query calculates: Average of (Payment Date - Invoice Date).
    
    let mut stmt = conn.prepare(
        "
        SELECT 
            AVG(julianday(p.document_date) - julianday(i.date)) as avg_days
        FROM invoices i
        JOIN payments p ON p.account_id = i.account_id
        WHERE i.account_id = ?1 
          AND i.type = 'sale' 
          AND p.type = 'receipt'
          AND p.document_date >= i.date
        "
    ).map_err(|e| e.to_string())?;

    let avg_days: f64 = stmt.query_row(params![account_id], |row| {
        Ok(row.get(0).unwrap_or(0.0))
    }).unwrap_or(0.0);

    // Get count of late payments by seeing if there are any overdue reminders that were resolved late or are pending
    let mut stmt2 = conn.prepare(
        "SELECT count(*) FROM reminders WHERE account_id = ?1 AND date(due_date) < date('now')"
    ).map_err(|e| e.to_string())?;

    let late_payments: i32 = stmt2.query_row(params![account_id], |row| {
        Ok(row.get(0).unwrap_or(0))
    }).unwrap_or(0);

    Ok(PaybackMetrics {
        average_days: avg_days.round(),
        late_payments,
    })
}
