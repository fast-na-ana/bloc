use crate::db::DbState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct DashboardRollup {
    pub cash_in_hand: f64,
    pub monthly_sales: f64,
    pub net_profit: f64,
    pub today_sales: f64,
    pub today_purchases: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DailyProfit {
    pub date: String,
    pub sales: f64,
    pub purchases: f64,
    pub profit: f64,
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

    let today_sales_paisa: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries
             WHERE account_code='4000'
               AND date(entry_date) = date('now', 'localtime')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let today_purchases_paisa: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries
             WHERE account_code='5000'
               AND date(entry_date) = date('now', 'localtime')",
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
        today_sales:    today_sales_paisa as f64 / 100.0,
        today_purchases:today_purchases_paisa as f64 / 100.0,
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

#[tauri::command]
pub fn get_daily_profits(state: State<'_, DbState>) -> Result<Vec<DailyProfit>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT 
                date(entry_date) as date,
                SUM(CASE WHEN account_code = '4000' THEN credit ELSE 0 END) - SUM(CASE WHEN account_code = '4100' THEN debit ELSE 0 END) as sales,
                SUM(CASE WHEN account_code = '5000' THEN debit ELSE 0 END) - SUM(CASE WHEN account_code = '5100' THEN credit ELSE 0 END) as purchases,
                SUM(CASE WHEN account_code IN ('4000', '5100') THEN credit ELSE 0 END) - SUM(CASE WHEN account_code IN ('4100', '5000', '6000', '6100', '9000') THEN debit ELSE 0 END) as profit
             FROM posted_ledger_entries
             WHERE strftime('%Y-%m', entry_date) = strftime('%Y-%m', 'now', 'localtime')
             GROUP BY date(entry_date)
             ORDER BY date ASC",
        )
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(DailyProfit {
                date: row.get(0)?,
                sales: row.get::<_, i64>(1)? as f64 / 100.0,
                purchases: row.get::<_, i64>(2)? as f64 / 100.0,
                profit: row.get::<_, i64>(3)? as f64 / 100.0,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct OverduePayer {
    pub name: String,
    pub days_old: i32,
    pub balance: f64,
}

#[tauri::command]
pub fn get_overdue_payers(state: State<'_, DbState>) -> Result<Vec<OverduePayer>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT a.name, CAST(julianday('now') - julianday(i.date) AS INTEGER) AS days_old, (i.total - COALESCE((SELECT SUM(amount) FROM payment_allocations WHERE invoice_id = i.id), 0)) as balance
             FROM invoices i
             JOIN accounts a ON a.id = i.account_id
             WHERE i.type = 'sale' AND i.voided_at IS NULL
             GROUP BY i.id
             HAVING balance > 0
             ORDER BY days_old DESC
             LIMIT 5",
        )
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(OverduePayer {
                name: row.get(0)?,
                days_old: row.get(1)?,
                balance: row.get::<_, i64>(2)? as f64 / 100.0,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct CashTransaction {
    pub description: String,
    pub amount: f64,
    pub date: String,
}

#[tauri::command]
pub fn get_recent_cash_transactions(state: State<'_, DbState>) -> Result<Vec<CashTransaction>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT je.description, le.debit, le.credit, le.entry_date
             FROM posted_ledger_entries le
             JOIN journal_entries je ON je.id = le.journal_entry_id
             WHERE le.account_code = '1000'
             ORDER BY le.entry_date DESC, je.id DESC
             LIMIT 5",
        )
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            let desc: String = row.get(0)?;
            let debit: i64 = row.get(1)?;
            let credit: i64 = row.get(2)?;
            let date: String = row.get(3)?;
            
            let amount = (debit as f64 - credit as f64) / 100.0;

            Ok(CashTransaction {
                description: desc,
                amount,
                date,
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
