use rusqlite::{Connection, params};
use std::path::PathBuf;

fn main() {
    let db_path = PathBuf::from(r"C:\Users\Abd\AppData\Roaming\com.abd.tauri-app\bloc.db");
    let conn = Connection::open(db_path).expect("Failed to open DB");

    // We assume the schema is already initialized by running the app once.

    // 1. Add some inventory items
    conn.execute(
        "INSERT INTO inventory (name, qty_on_hand, total_value_paisa, cost_price, sale_price) VALUES 
        ('Premium Widget A', 50, 500000, 10000, 15000),
        ('Deluxe Gadget B', 25, 375000, 15000, 25000),
        ('Basic Tool C', 100, 500000, 5000, 8000)",
        [],
    ).ok();

    // 2. Add some accounts (Customer & Supplier)
    conn.execute(
        "INSERT INTO accounts (name, type, credit_limit) VALUES 
        ('Acme Corp', 'customer', 5000000),
        ('Global Tech', 'customer', 2000000),
        ('Supplies Co', 'supplier', 1000000)",
        [],
    ).ok();

    // 3. Add some expenses for today
    conn.execute(
        "INSERT INTO expenses (description, amount, date) VALUES 
        ('Office Supplies', 12500, date('now')),
        ('Internet Bill', 6000, date('now')),
        ('Maintenance', 15000, date('now'))",
        [],
    ).ok();

    // 4. Add some invoices (sales & purchases) for today
    // Insert sale invoice
    conn.execute(
        "INSERT INTO invoices (type, account_id, date, total, discount) VALUES 
        ('sale', 1, date('now'), 45000, 0),
        ('purchase', 3, date('now'), 15000, 0)",
        [],
    ).ok();

    // Insert invoice items
    // Invoice 1 (sale)
    conn.execute(
        "INSERT INTO invoice_items (invoice_id, inventory_id, quantity, rate, discount, total, cost_at_sale) VALUES 
        (1, 1, 3, 15000, 0, 45000, 10000)",
        [],
    ).ok();
    // Invoice 2 (purchase)
    conn.execute(
        "INSERT INTO invoice_items (invoice_id, inventory_id, quantity, rate, discount, total) VALUES 
        (2, 2, 1, 15000, 0, 15000)",
        [],
    ).ok();

    // 5. Add some payments (cash in and out) for today
    conn.execute(
        "INSERT INTO payments (account_id, type, amount, tid, document_date, settlement_account_code) VALUES 
        (1, 'receive', 20000, 'TID-001', date('now'), '1000'),
        (3, 'pay', 5000, 'TID-002', date('now'), '1000')",
        [],
    ).ok();

    println!("Seed completed successfully.");
}
