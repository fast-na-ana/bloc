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
pub fn get_account_ledger(
    state: State<'_, DbState>,
    account_id: i64,
    _role: String, // Kept to avoid changing the frontend signature for now, though we will change frontend too
) -> Result<Vec<LedgerEntry>, String> {
    let conn = state.0.lock().unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT le.entry_date, je.description, le.debit, le.credit
             FROM posted_ledger_entries le
             JOIN journal_entries je ON je.id = le.journal_entry_id
             WHERE le.account_code IN ('1100', '2000') AND le.party_id = ?1
             ORDER BY le.entry_date ASC, le.journal_entry_id ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, String, i64, i64)> = stmt
        .query_map(params![account_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut ledger = Vec::new();
    let mut running_paisa: i64 = 0;

    for (date, description, debit_paisa, credit_paisa) in rows {
        // AR and AP are both debit-normal from the perspective of the control account,
        // but the party's outstanding balance increases on the debit side (customer owes us)
        // and decreases on the credit side (payment received / return).
        // For suppliers the same logic applies in reverse but the query already gives us
        // the correct sign from the AP (2000 credit-normal) entries:
        //   purchase → Cr 2000 appears as credit here (balance goes up)
        //   payment  → Dr 2000 appears as debit here  (balance goes down)
        running_paisa += debit_paisa - credit_paisa;
        if debit_paisa > 0 || credit_paisa > 0 {
            ledger.push(LedgerEntry {
                date,
                description,
                debit:   debit_paisa as f64 / 100.0,
                credit:  credit_paisa as f64 / 100.0,
                balance: running_paisa as f64 / 100.0,
            });
        }
    }

    Ok(ledger)
}

#[tauri::command]
pub fn get_item_ledger(
    state: State<'_, DbState>,
    inventory_id: i64,
) -> Result<Vec<ItemLedgerEntry>, String> {
    let conn = state.0.lock().unwrap();

    // Item ledger is a physical-movement view (qty, not money).
    // Reads from invoice_items; filters voided invoices so voided documents are invisible.
    let mut stmt = conn
        .prepare(
            "SELECT i.date, i.type, it.quantity
             FROM invoices i
             JOIN invoice_items it ON i.id = it.invoice_id
             WHERE it.inventory_id = ?1
               AND i.voided_at IS NULL
             ORDER BY i.date ASC, i.id ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, String, f64)> = stmt
        .query_map(params![inventory_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut ledger = Vec::new();
    let mut running_balance = 0.0_f64;

    for (date, inv_type, qty) in rows {
        let (q_in, q_out) = match inv_type.as_str() {
            "purchase" | "sale_return"  => (qty, 0.0),
            "sale"     | "purchase_return" => (0.0, qty),
            _ => (0.0, 0.0),
        };
        running_balance += q_in - q_out;
        ledger.push(ItemLedgerEntry {
            date,
            description: inv_type,
            quantity_in: q_in,
            quantity_out: q_out,
            balance: running_balance,
        });
    }

    Ok(ledger)
}
