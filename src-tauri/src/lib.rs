pub mod db;
pub mod migrations;
pub mod backup;
pub mod invoices;
pub mod expenses;
pub mod ledger;
pub mod payments;
pub mod analytics;
pub mod assets;
pub mod auth;
pub mod journal;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let conn = db::init_db(app.handle())
                .expect("Failed to initialize database");
                
            app.manage(db::DbState(std::sync::Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            backup::backup_db,
            backup::restore_db,
            invoices::get_accounts,
            invoices::get_inventory,
            invoices::create_invoice,
            invoices::void_invoice,
            invoices::create_account,
            invoices::create_inventory_item,
            invoices::update_credit_limit,
            invoices::get_account_summaries,
            analytics::analyze_payback_behavior,
            expenses::create_expense,
            expenses::get_expenses,
            expenses::create_reminder,
            expenses::get_reminders,
            ledger::get_account_ledger,
            ledger::get_item_ledger,
            payments::create_payment,
            payments::get_payments,
            analytics::get_financial_dashboard,
            analytics::get_income_statement,
            analytics::get_balance_sheet,
            assets::create_asset,
            assets::get_assets,
            auth::check_has_users,
            auth::setup_admin,
            auth::login,
            auth::create_user,
            journal::post_manual_journal
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
