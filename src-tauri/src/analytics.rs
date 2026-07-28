use crate::db::DbState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct DashboardRollup {
    pub cash_in_hand: f64,
    pub monthly_sales: f64,
    pub net_profit: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IncomeStatement {
    pub revenue: f64,
    pub cogs: f64,
    pub gross_profit: f64,
    pub operating_expenses: f64,
    pub net_profit: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BalanceSheet {
    pub cash: f64,
    pub inventory_value: f64,
    pub fixed_assets: f64,
    pub accounts_receivable: f64,
    pub accounts_payable: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaybackMetrics {
    pub average_days: f64,
    pub late_payments: i32,
}

fn ledger_balance(conn: &rusqlite::Connection, account_code: &str, side: &str) -> i64 {
    let sql = if side == "debit" {
        format!(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='{}'",
            account_code
        )
    } else {
        format!(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries WHERE account_code='{}'",
            account_code
        )
    };
    conn.query_row(&sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0)
}

#[tauri::command]
pub fn get_financial_dashboard(state: State<'_, DbState>) -> Result<DashboardRollup, String> {
    let conn = state.0.lock().unwrap();

    let cash_paisa = ledger_balance(&conn, "1000", "debit");

    let monthly_sales_paisa: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries
             WHERE account_code='4000'
               AND strftime('%Y-%m', entry_date) = strftime('%Y-%m', 'now')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Net profit = (revenue - sales_returns) - (cogs - purchase_returns) - opex - depr - rounding
    let revenue    = ledger_balance(&conn, "4000", "credit");
    let sal_ret    = ledger_balance(&conn, "4100", "debit");
    let cogs       = ledger_balance(&conn, "5000", "debit");
    let pur_ret    = ledger_balance(&conn, "5100", "credit");
    let opex       = ledger_balance(&conn, "6000", "debit");
    let depr       = ledger_balance(&conn, "6100", "debit");
    let rounding   = ledger_balance(&conn, "9000", "debit"); // negative when Cr (income)
    let net_profit_paisa = revenue - sal_ret - (cogs - pur_ret) - opex - depr - rounding;

    Ok(DashboardRollup {
        cash_in_hand:   cash_paisa as f64 / 100.0,
        monthly_sales:  monthly_sales_paisa as f64 / 100.0,
        net_profit:     net_profit_paisa as f64 / 100.0,
    })
}

#[tauri::command]
pub fn get_income_statement(state: State<'_, DbState>) -> Result<IncomeStatement, String> {
    let conn = state.0.lock().unwrap();

    let revenue_paisa = ledger_balance(&conn, "4000", "credit")
        - ledger_balance(&conn, "4100", "debit");       // net of sales returns
    let cogs_paisa = ledger_balance(&conn, "5000", "debit")
        - ledger_balance(&conn, "5100", "credit");      // net of purchase returns
    let gross_profit_paisa = revenue_paisa - cogs_paisa;
    let opex_paisa = ledger_balance(&conn, "6000", "debit")
        + ledger_balance(&conn, "6100", "debit");       // operating + depreciation
    let net_profit_paisa = gross_profit_paisa - opex_paisa;

    Ok(IncomeStatement {
        revenue:              revenue_paisa as f64 / 100.0,
        cogs:                 cogs_paisa as f64 / 100.0,
        gross_profit:         gross_profit_paisa as f64 / 100.0,
        operating_expenses:   opex_paisa as f64 / 100.0,
        net_profit:           net_profit_paisa as f64 / 100.0,
    })
}

#[tauri::command]
pub fn get_balance_sheet(state: State<'_, DbState>) -> Result<BalanceSheet, String> {
    let conn = state.0.lock().unwrap();

    let cash_paisa      = ledger_balance(&conn, "1000", "debit");
    let ar_paisa        = ledger_balance(&conn, "1100", "debit");
    let inv_paisa       = ledger_balance(&conn, "1200", "debit");
    let fa_paisa        = ledger_balance(&conn, "1300", "debit");
    let accum_depr_paisa= ledger_balance(&conn, "1310", "credit");
    let ap_paisa        = ledger_balance(&conn, "2000", "credit");

    Ok(BalanceSheet {
        cash:                 cash_paisa as f64 / 100.0,
        inventory_value:      inv_paisa as f64 / 100.0,
        fixed_assets:         (fa_paisa - accum_depr_paisa) as f64 / 100.0,
        accounts_receivable:  ar_paisa.max(0) as f64 / 100.0,
        accounts_payable:     ap_paisa.max(0) as f64 / 100.0,
    })
}

#[tauri::command]
pub fn analyze_payback_behavior(
    state: State<'_, DbState>,
    account_id: i64,
) -> Result<PaybackMetrics, String> {
    let conn = state.0.lock().unwrap();

    // Matched-pairs join through payment_allocations — no Cartesian product.
    // average days from invoice date to the receipt that settled it.
    let avg_days: f64 = conn
        .query_row(
            "SELECT AVG(julianday(p.document_date) - julianday(i.date))
             FROM payment_allocations pa
             JOIN payments p ON p.id = pa.payment_id
             JOIN invoices i ON i.id = pa.invoice_id
             WHERE i.account_id = ?1
               AND i.type = 'sale'
               AND p.type = 'receipt'
               AND p.document_date >= i.date
               AND p.voided_at IS NULL
               AND i.voided_at IS NULL",
            rusqlite::params![account_id],
            |r| Ok(r.get::<_, f64>(0).unwrap_or(0.0)),
        )
        .unwrap_or(0.0);

    let late_payments: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM reminders
             WHERE account_id = ?1 AND date(due_date) < date('now')",
            rusqlite::params![account_id],
            |r| Ok(r.get::<_, i32>(0).unwrap_or(0)),
        )
        .unwrap_or(0);

    Ok(PaybackMetrics {
        average_days: avg_days.round(),
        late_payments,
    })
}
