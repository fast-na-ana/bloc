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

#[tauri::command]
pub fn get_financial_dashboard(state: State<'_, DbState>) -> Result<DashboardRollup, String> {
    let conn = state.0.lock().unwrap();
    
    // Cash in Hand: (Total Receipts) - (Total Payments) - (Total Expenses)
    let receipts: f64 = conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM payments WHERE type = 'receipt'", [], |r| r.get(0)).unwrap_or(0.0);
    let payments: f64 = conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM payments WHERE type = 'payment'", [], |r| r.get(0)).unwrap_or(0.0);
    let expenses: f64 = conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM expenses", [], |r| r.get(0)).unwrap_or(0.0);
    let cash_in_hand = receipts - payments - expenses;

    // Monthly Sales (Current month)
    let monthly_sales: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total), 0) FROM invoices WHERE type = 'sale' AND strftime('%Y-%m', date) = strftime('%Y-%m', 'now')",
        [], |r| r.get(0)
    ).unwrap_or(0.0);

    // Net Profit: (Total Sales) - (Total COGS) - (Total Expenses)
    let total_sales: f64 = conn.query_row("SELECT COALESCE(SUM(total), 0) FROM invoices WHERE type = 'sale'", [], |r| r.get(0)).unwrap_or(0.0);
    let cogs: f64 = conn.query_row(
        "SELECT COALESCE(SUM(ii.quantity * inv.cost_price), 0) FROM invoice_items ii 
         JOIN invoices i ON i.id = ii.invoice_id 
         JOIN inventory inv ON inv.id = ii.inventory_id 
         WHERE i.type = 'sale'",
        [], |r| r.get(0)
    ).unwrap_or(0.0);
    let net_profit = total_sales - cogs - expenses;

    Ok(DashboardRollup {
        cash_in_hand,
        monthly_sales,
        net_profit,
    })
}

#[tauri::command]
pub fn get_income_statement(state: State<'_, DbState>) -> Result<IncomeStatement, String> {
    let conn = state.0.lock().unwrap();
    
    let revenue: f64 = conn.query_row("SELECT COALESCE(SUM(total), 0) FROM invoices WHERE type = 'sale'", [], |r| r.get(0)).unwrap_or(0.0);
    let cogs: f64 = conn.query_row(
        "SELECT COALESCE(SUM(ii.quantity * inv.cost_price), 0) FROM invoice_items ii 
         JOIN invoices i ON i.id = ii.invoice_id 
         JOIN inventory inv ON inv.id = ii.inventory_id 
         WHERE i.type = 'sale'",
        [], |r| r.get(0)
    ).unwrap_or(0.0);
    
    let gross_profit = revenue - cogs;
    let operating_expenses: f64 = conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM expenses", [], |r| r.get(0)).unwrap_or(0.0);
    let net_profit = gross_profit - operating_expenses;

    Ok(IncomeStatement {
        revenue,
        cogs,
        gross_profit,
        operating_expenses,
        net_profit,
    })
}

#[tauri::command]
pub fn get_balance_sheet(state: State<'_, DbState>) -> Result<BalanceSheet, String> {
    let conn = state.0.lock().unwrap();
    
    // Cash
    let receipts: f64 = conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM payments WHERE type = 'receipt'", [], |r| r.get(0)).unwrap_or(0.0);
    let payments: f64 = conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM payments WHERE type = 'payment'", [], |r| r.get(0)).unwrap_or(0.0);
    let expenses: f64 = conn.query_row("SELECT COALESCE(SUM(amount), 0) FROM expenses", [], |r| r.get(0)).unwrap_or(0.0);
    let cash = receipts - payments - expenses;

    // Inventory Value
    let inventory_value: f64 = conn.query_row("SELECT COALESCE(SUM(stock_level * cost_price), 0) FROM inventory", [], |r| r.get(0)).unwrap_or(0.0);

    // Fixed Assets (Value - Depreciation)
    // Straight line dynamic depreciation for simplicity
    let fixed_assets: f64 = conn.query_row("SELECT COALESCE(SUM(value * (1.0 - (depreciation_rate / 100.0))), 0) FROM assets", [], |r| r.get(0)).unwrap_or(0.0);

    // Accounts Receivable: (Total Sales) - (Total Receipts for Customers)
    let sales: f64 = conn.query_row("SELECT COALESCE(SUM(total), 0) FROM invoices WHERE type = 'sale'", [], |r| r.get(0)).unwrap_or(0.0);
    // Accounts Payable: (Total Purchases) - (Total Payments for Suppliers)
    let purchases: f64 = conn.query_row("SELECT COALESCE(SUM(total), 0) FROM invoices WHERE type = 'purchase'", [], |r| r.get(0)).unwrap_or(0.0);

    let accounts_receivable = sales - receipts;
    let accounts_payable = purchases - payments;

    Ok(BalanceSheet {
        cash,
        inventory_value,
        fixed_assets,
        accounts_receivable: if accounts_receivable > 0.0 { accounts_receivable } else { 0.0 },
        accounts_payable: if accounts_payable > 0.0 { accounts_payable } else { 0.0 },
    })
}
