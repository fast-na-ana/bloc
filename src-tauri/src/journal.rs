use crate::db::DbState;
use rusqlite::{params, Transaction};
#[cfg(test)]
use rusqlite::Connection;
use serde::Deserialize;
use tauri::State;

// A single debit or credit line for a journal entry.
pub struct JournalLine {
    pub account_code: String,
    pub party_id: Option<i64>,
    pub debit: i64,  // paisa; exactly one of debit/credit must be non-zero
    pub credit: i64, // paisa
}

// Creates a journal entry with all its lines atomically, then posts it.
// The balance trigger fires when is_posted flips to 1.
// Returns the new journal_entry.id on success.
pub fn post_journal_entry(
    tx: &Transaction,
    entry_date: &str,
    description: &str,
    reference_type: &str,
    reference_id: Option<i64>,
    reverses_id: Option<i64>,
    created_by: i64,
    lines: &[JournalLine],
) -> Result<i64, String> {
    tx.execute(
        "INSERT INTO journal_entries (entry_date, description, reference_type, reference_id, reverses_journal_entry_id, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![entry_date, description, reference_type, reference_id, reverses_id, created_by],
    )
    .map_err(|e| e.to_string())?;

    let je_id = tx.last_insert_rowid();

    for line in lines {
        tx.execute(
            "INSERT INTO ledger_entries (journal_entry_id, account_code, party_id, debit, credit)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![je_id, line.account_code, line.party_id, line.debit, line.credit],
        )
        .map_err(|e| e.to_string())?;
    }

    // Flip is_posted — balance trigger runs here
    tx.execute(
        "UPDATE journal_entries SET is_posted = 1 WHERE id = ?1",
        params![je_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(je_id)
}

// Payload for posting a manual journal entry from the frontend (admin-only adjustment).
#[derive(Deserialize)]
pub struct ManualJournalLine {
    pub account_code: String,
    pub party_id: Option<i64>,
    pub debit: f64,  // frontend passes display units (rupees); converted to paisa here
    pub credit: f64,
}

#[tauri::command]
pub fn post_manual_journal(
    state: State<'_, DbState>,
    entry_date: String,
    description: String,
    lines: Vec<ManualJournalLine>,
    user_id: i64,
) -> Result<i64, String> {
    let mut conn = state.0.lock().unwrap();
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    let journal_lines: Vec<JournalLine> = lines
        .iter()
        .map(|l| JournalLine {
            account_code: l.account_code.clone(),
            party_id: l.party_id,
            debit: (l.debit * 100.0).round() as i64,
            credit: (l.credit * 100.0).round() as i64,
        })
        .collect();

    let je_id = post_journal_entry(
        &tx,
        &entry_date,
        &description,
        "adjustment",
        None,
        None,
        user_id,
        &journal_lines,
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(je_id)
}

#[cfg(test)]
pub fn open_test_db() -> Connection {
    use crate::migrations;
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    migrations::run(&conn).expect("migrations failed");
    // Seed a test user (required by journal_entries.created_by FK)
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role) VALUES (1, 'test', 'x', 'admin')",
        [],
    )
    .unwrap();
    conn
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn sum_debits(conn: &Connection, je_id: i64) -> i64 {
        conn.query_row(
            "SELECT COALESCE(SUM(debit), 0) FROM ledger_entries WHERE journal_entry_id = ?1",
            params![je_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn sum_credits(conn: &Connection, je_id: i64) -> i64 {
        conn.query_row(
            "SELECT COALESCE(SUM(credit), 0) FROM ledger_entries WHERE journal_entry_id = ?1",
            params![je_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn is_posted(conn: &Connection, je_id: i64) -> bool {
        conn.query_row(
            "SELECT is_posted FROM journal_entries WHERE id = ?1",
            params![je_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    fn count_in_view(conn: &Connection, je_id: i64) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM posted_ledger_entries WHERE journal_entry_id = ?1",
            params![je_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn balanced_entry_posts_successfully() {
        let mut conn = open_test_db();
        // Seed required account chart entries
        conn.execute_batch(
            "INSERT OR IGNORE INTO accounts_chart (code,name,account_type,normal_balance) VALUES
             ('1100','AR','asset','debit'),
             ('4000','Revenue','revenue','credit');",
        )
        .unwrap();

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();

        let lines = vec![
            JournalLine { account_code: "1100".into(), party_id: None, debit: 50000, credit: 0 },
            JournalLine { account_code: "4000".into(), party_id: None, debit: 0, credit: 50000 },
        ];
        let je_id = post_journal_entry(&tx, "2026-01-01", "test sale", "invoice", Some(1), None, 1, &lines).unwrap();
        tx.commit().unwrap();

        assert!(is_posted(&conn, je_id));
        assert_eq!(sum_debits(&conn, je_id), sum_credits(&conn, je_id));
        assert_eq!(count_in_view(&conn, je_id), 2);
    }

    #[test]
    fn unbalanced_entry_is_rejected() {
        let mut conn = open_test_db();
        conn.execute_batch(
            "INSERT OR IGNORE INTO accounts_chart (code,name,account_type,normal_balance) VALUES
             ('1100','AR','asset','debit'),
             ('4000','Revenue','revenue','credit');",
        )
        .unwrap();

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();

        let lines = vec![
            JournalLine { account_code: "1100".into(), party_id: None, debit: 50000, credit: 0 },
            JournalLine { account_code: "4000".into(), party_id: None, debit: 0, credit: 40000 }, // intentionally wrong
        ];
        let result = post_journal_entry(&tx, "2026-01-01", "bad entry", "invoice", Some(1), None, 1, &lines);
        assert!(result.is_err(), "Expected error for unbalanced entry");
        assert!(result.unwrap_err().contains("balance"));
    }

    #[test]
    fn posted_lines_are_immutable() {
        let mut conn = open_test_db();
        conn.execute_batch(
            "INSERT OR IGNORE INTO accounts_chart (code,name,account_type,normal_balance) VALUES
             ('1100','AR','asset','debit'),
             ('4000','Revenue','revenue','credit');",
        )
        .unwrap();

        let (je_id, line_id) = {
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .unwrap();
            let lines = vec![
                JournalLine { account_code: "1100".into(), party_id: None, debit: 10000, credit: 0 },
                JournalLine { account_code: "4000".into(), party_id: None, debit: 0, credit: 10000 },
            ];
            let id = post_journal_entry(&tx, "2026-01-01", "immutability test", "invoice", Some(1), None, 1, &lines).unwrap();
            let line_id: i64 = tx.query_row(
                "SELECT id FROM ledger_entries WHERE journal_entry_id = ?1 LIMIT 1",
                params![id],
                |r| r.get(0),
            ).unwrap();
            tx.commit().unwrap();
            (id, line_id)
        };

        // Attempt to UPDATE a posted line — must fail
        let update_result = conn.execute(
            "UPDATE ledger_entries SET debit = 99999 WHERE id = ?1",
            params![line_id],
        );
        assert!(update_result.is_err(), "UPDATE on posted line should be blocked");

        // Attempt to DELETE a posted line — must fail
        let delete_result = conn.execute(
            "DELETE FROM ledger_entries WHERE id = ?1",
            params![line_id],
        );
        assert!(delete_result.is_err(), "DELETE on posted line should be blocked");

        // Original values must be unchanged
        assert_eq!(sum_debits(&conn, je_id), 10000);
    }

    #[test]
    fn unposted_entries_invisible_to_view() {
        let conn = open_test_db();
        conn.execute_batch(
            "INSERT OR IGNORE INTO accounts_chart (code,name,account_type,normal_balance) VALUES
             ('1100','AR','asset','debit'),
             ('4000','Revenue','revenue','credit');",
        )
        .unwrap();

        // Insert a journal entry + lines manually WITHOUT posting
        conn.execute(
            "INSERT INTO journal_entries (entry_date, description, reference_type, created_by)
             VALUES ('2026-01-01', 'unposted', 'adjustment', 1)",
            [],
        ).unwrap();
        let je_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO ledger_entries (journal_entry_id, account_code, debit, credit) VALUES (?1, '1100', 10000, 0)",
            params![je_id],
        ).unwrap();

        // Must not appear in view
        assert_eq!(count_in_view(&conn, je_id), 0, "Unposted entry must not appear in posted_ledger_entries");
    }

    #[test]
    fn double_post_is_rejected() {
        let mut conn = open_test_db();
        conn.execute_batch(
            "INSERT OR IGNORE INTO accounts_chart (code,name,account_type,normal_balance) VALUES
             ('1100','AR','asset','debit'),
             ('4000','Revenue','revenue','credit');",
        )
        .unwrap();

        let je_id = {
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .unwrap();
            let lines = vec![
                JournalLine { account_code: "1100".into(), party_id: None, debit: 5000, credit: 0 },
                JournalLine { account_code: "4000".into(), party_id: None, debit: 0, credit: 5000 },
            ];
            let id = post_journal_entry(&tx, "2026-01-01", "double post test", "invoice", Some(1), None, 1, &lines).unwrap();
            tx.commit().unwrap();
            id
        };

        // Try posting again
        let result = conn.execute(
            "UPDATE journal_entries SET is_posted = 1 WHERE id = ?1",
            params![je_id],
        );
        assert!(result.is_err(), "Double-post should be rejected by trigger");
    }

    // ── accounting correctness tests ────────────────────────────────────────────

    fn seed_customer(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO accounts (name, type, credit_limit) VALUES ('Test Customer', 'customer', 0)",
            [],
        ).unwrap();
        conn.last_insert_rowid()
    }

    fn seed_supplier(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO accounts (name, type, credit_limit) VALUES ('Test Supplier', 'supplier', 0)",
            [],
        ).unwrap();
        conn.last_insert_rowid()
    }

    fn seed_inventory(conn: &Connection, qty: i64, cost_paisa: i64) -> i64 {
        let total_val = qty * cost_paisa;
        conn.execute(
            "INSERT INTO inventory (name, qty_on_hand, total_value_paisa, cost_price, sale_price)
             VALUES ('Widget', ?1, ?2, ?3, ?4)",
            params![qty, total_val, cost_paisa, cost_paisa * 2],
        ).unwrap();
        conn.last_insert_rowid()
    }

    fn ledger_lines(conn: &Connection, je_id: i64) -> Vec<(String, Option<i64>, i64, i64)> {
        let mut stmt = conn.prepare(
            "SELECT account_code, party_id, debit, credit FROM ledger_entries
             WHERE journal_entry_id = ?1 ORDER BY id"
        ).unwrap();
        stmt.query_map(params![je_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap().filter_map(|r| r.ok()).collect()
    }

    fn last_je_id(conn: &Connection) -> i64 {
        conn.query_row("SELECT MAX(id) FROM journal_entries", [], |r| r.get(0)).unwrap()
    }

    fn ar_balance(conn: &Connection, party_id: i64) -> i64 {
        conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries
             WHERE account_code='1100' AND party_id=?1",
            params![party_id], |r| r.get(0)
        ).unwrap()
    }

    fn inventory_row(conn: &Connection, inv_id: i64) -> (i64, i64) {
        conn.query_row(
            "SELECT qty_on_hand, total_value_paisa FROM inventory WHERE id=?1",
            params![inv_id], |r| Ok((r.get(0)?, r.get(1)?))
        ).unwrap()
    }

    // Test 1: Credit sale posts correct journal entries
    #[test]
    fn t1_credit_sale_posts_correct_entries() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000); // 100 units @ ₹10.00

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};

        let payload = InvoiceData {
            invoice_type: "sale".into(),
            account_id: cust_id,
            date: "2026-01-15".into(),
            total: 150.00,   // 10 units × ₹15.00
            discount: 0.0,
            items: vec![InvoiceItemData {
                inventory_id: inv_id,
                quantity: 10.0,
                rate: 15.00,
                discount: 0.0,
                total: 150.00,
            }],
            user_id: Some(1),
        };

        create_invoice_inner(&mut conn, payload).expect("create_invoice_inner failed");
        let je_id = last_je_id(&conn);
        let lines = ledger_lines(&conn, je_id);

        println!("T1 ledger lines:");
        for (code, party, dr, cr) in &lines {
            println!("  {} party={:?} Dr={} Cr={}", code, party, dr, cr);
        }

        // AR: Dr 1100, 15000 paisa, party=customer
        assert!(lines.iter().any(|(c,p,dr,cr)| c=="1100" && *p==Some(cust_id) && *dr==15000 && *cr==0),
            "Missing Dr 1100 AR 15000 for customer");
        // Revenue: Cr 4000, 15000 paisa
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="4000" && *dr==0 && *cr==15000),
            "Missing Cr 4000 Revenue 15000");
        // COGS: Dr 5000, 10000 paisa (WAC=1000 × 10 units)
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="5000" && *dr==10000 && *cr==0),
            "Missing Dr 5000 COGS 10000");
        // Inventory: Cr 1200, 10000 paisa
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="1200" && *dr==0 && *cr==10000),
            "Missing Cr 1200 Inventory 10000");

        // Inventory updated correctly
        let (qty, val) = inventory_row(&conn, inv_id);
        assert_eq!(qty, 90, "qty_on_hand should be 90");
        assert_eq!(val, 90000, "total_value_paisa should be 90000");
    }

    // Test 2: Cash sale rounding — 1-paisa difference should go to 9000
    // EXPECTED FAILURE: create_invoice always posts to AR (1100), not cash (1000),
    // and has no rounding-difference path to 9000.
    #[test]
    fn t2_cash_sale_rounding_to_9000() {
        // Scenario: 3 units with WAC = 333.33... paisa (total_value=1000, qty=3)
        // COGS = ROUND(1000/3) * 3 = 333 * 3 = 999  (1 paisa stranded in inventory)
        // Invoice total entered as ₹30.00 (3 units × ₹10.00)
        // A cash sale should post Dr 1000 Cash / Cr 4000 Revenue, not Dr 1100 AR
        // The 1-paisa COGS/inventory mismatch should create a Dr/Cr 9000 entry
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        // Seed 3 units with total_value_paisa=1000 (not divisible by 3)
        conn.execute(
            "INSERT INTO inventory (name, qty_on_hand, total_value_paisa, cost_price, sale_price)
             VALUES ('Odd Item', 3, 1000, 334, 1000)",
            [],
        ).unwrap();
        let inv_id = conn.last_insert_rowid();

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};
        let payload = InvoiceData {
            invoice_type: "sale".into(),
            account_id: cust_id,
            date: "2026-01-15".into(),
            total: 30.00,
            discount: 0.0,
            items: vec![InvoiceItemData {
                inventory_id: inv_id, quantity: 3.0, rate: 10.00, discount: 0.0, total: 30.00,
            }],
            user_id: Some(1),
        };
        create_invoice_inner(&mut conn, payload).expect("sale should succeed");
        let je_id = last_je_id(&conn);
        let lines = ledger_lines(&conn, je_id);

        println!("T2 ledger lines (cash sale rounding test):");
        for (code, party, dr, cr) in &lines {
            println!("  {} party={:?} Dr={} Cr={}", code, party, dr, cr);
        }

        // EXPECTED FAILURE: should post to 1000 (Cash), not 1100 (AR)
        let has_cash_leg = lines.iter().any(|(c,_,dr,_)| c=="1000" && *dr==3000);
        assert!(has_cash_leg,
            "FAIL: Cash sale should Dr 1000 Cash 3000, but currently posts Dr 1100 AR. \
             Cash sale path not implemented — always goes through AR.");

        // EXPECTED FAILURE: 1 paisa rounding difference should go to 9000
        // WAC=333, COGS=999, but inventory had 1000 — 1 paisa difference
        let has_rounding = lines.iter().any(|(c,_,_,_)| c=="9000");
        assert!(has_rounding,
            "FAIL: 1-paisa WAC rounding difference (1000 - 999) should post to 9000, but no 9000 entry exists.");
    }

    // Test 3: Purchase invoice increases WAC correctly
    #[test]
    fn t3_purchase_increases_wac_correctly() {
        let mut conn = open_test_db();
        let sup_id = seed_supplier(&conn);
        conn.execute(
            "INSERT INTO inventory (name, qty_on_hand, total_value_paisa, cost_price, sale_price)
             VALUES ('Widget', 0, 0, 0, 0)",
            [],
        ).unwrap();
        let inv_id = conn.last_insert_rowid();

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};
        let payload = InvoiceData {
            invoice_type: "purchase".into(),
            account_id: sup_id,
            date: "2026-01-01".into(),
            total: 1000.00,
            discount: 0.0,
            items: vec![InvoiceItemData {
                inventory_id: inv_id, quantity: 100.0, rate: 10.00, discount: 0.0, total: 1000.00,
            }],
            user_id: Some(1),
        };
        create_invoice_inner(&mut conn, payload).expect("purchase should succeed");

        let (qty, val) = inventory_row(&conn, inv_id);
        assert_eq!(qty, 100, "qty_on_hand should be 100");
        assert_eq!(val, 100000, "total_value_paisa should be 100000 (₹1000.00)");

        let je_id = last_je_id(&conn);
        let lines = ledger_lines(&conn, je_id);
        println!("T3 ledger lines:"); for (c,p,dr,cr) in &lines { println!("  {} {:?} Dr={} Cr={}", c,p,dr,cr); }

        assert!(lines.iter().any(|(c,_,dr,cr)| c=="1200" && *dr==100000 && *cr==0),
            "Missing Dr 1200 Inventory 100000");
        assert!(lines.iter().any(|(c,p,dr,cr)| c=="2000" && *p==Some(sup_id) && *dr==0 && *cr==100000),
            "Missing Cr 2000 AP 100000 for supplier");
    }

    // Test 4: cost_at_sale equals WAC at moment of sale; later purchase doesn't change it
    #[test]
    fn t4_wac_cost_at_sale_persists_after_later_purchase() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let sup_id = seed_supplier(&conn);
        conn.execute(
            "INSERT INTO inventory (name, qty_on_hand, total_value_paisa, cost_price, sale_price)
             VALUES ('Widget', 0, 0, 0, 0)",
            [],
        ).unwrap();
        let inv_id = conn.last_insert_rowid();

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};

        let make_purchase = |conn: &mut rusqlite::Connection, sup: i64, qty: f64, rate: f64, total: f64| {
            let p = InvoiceData {
                invoice_type: "purchase".into(), account_id: sup,
                date: "2026-01-01".into(), total, discount: 0.0,
                items: vec![InvoiceItemData { inventory_id: inv_id, quantity: qty, rate, discount: 0.0, total }],
                user_id: Some(1),
            };
            create_invoice_inner(conn, p).unwrap();
        };

        // Purchase 1: 100 @ ₹10.00 → total_value=100000, qty=100
        make_purchase(&mut conn, sup_id, 100.0, 10.00, 1000.00);
        // Purchase 2: 50 @ ₹14.00 → total_value=170000, qty=150
        make_purchase(&mut conn, sup_id, 50.0, 14.00, 700.00);
        // WAC = 170000/150 = 1133.33... → rounds to 1133 paisa

        let (qty_before, val_before) = inventory_row(&conn, inv_id);
        assert_eq!(qty_before, 150);
        assert_eq!(val_before, 170000);

        // Sell 10 units @ ₹20.00
        let sale = InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-05".into(), total: 200.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 20.00, discount: 0.0, total: 200.00 }],
            user_id: Some(1),
        };
        create_invoice_inner(&mut conn, sale).expect("sale should succeed");

        // cost_at_sale stored on invoice_items row
        let cost_at_sale: Option<i64> = conn.query_row(
            "SELECT cost_at_sale FROM invoice_items WHERE inventory_id=?1 ORDER BY id DESC LIMIT 1",
            params![inv_id], |r| r.get(0)
        ).unwrap();
        println!("T4 cost_at_sale={:?} (expected 1133)", cost_at_sale);
        assert_eq!(cost_at_sale, Some(1133), "cost_at_sale should be WAC = ROUND(170000/150) = 1133");

        // COGS in ledger = 1133 × 10 = 11330
        let je_id = last_je_id(&conn);
        let lines = ledger_lines(&conn, je_id);
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="5000" && *dr==11330 && *cr==0),
            "COGS should be 11330 (1133 × 10)");

        // Now purchase 3: 100 @ ₹8.00 — this should NOT change cost_at_sale of the earlier sale
        make_purchase(&mut conn, sup_id, 100.0, 8.00, 800.00);

        let cost_at_sale_after: Option<i64> = conn.query_row(
            "SELECT cost_at_sale FROM invoice_items WHERE inventory_id=?1 ORDER BY id DESC LIMIT 1",
            params![inv_id], |r| r.get(0)
        ).unwrap();
        // The "last" invoice_item is now the purchase, which has cost_at_sale=NULL
        // Find specifically the sale item
        let sale_cost_at_sale: Option<i64> = conn.query_row(
            "SELECT ii.cost_at_sale FROM invoice_items ii
             JOIN invoices i ON i.id = ii.invoice_id
             WHERE ii.inventory_id=?1 AND i.type='sale' ORDER BY ii.id DESC LIMIT 1",
            params![inv_id], |r| r.get(0)
        ).unwrap();
        println!("T4 cost_at_sale after later purchase={:?} (must still be 1133)", sale_cost_at_sale);
        assert_eq!(sale_cost_at_sale, Some(1133),
            "cost_at_sale must be immutable — later purchase must not change it");
        let _ = cost_at_sale_after;
    }

    // Test 5: Sale return should reverse COGS (currently missing)
    #[test]
    fn t5_sale_return_reverses_cogs() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000); // 100 units @ ₹10.00

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};

        // Sell 10 units
        let sale = InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-15".into(), total: 150.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 15.00, discount: 0.0, total: 150.00 }],
            user_id: Some(1),
        };
        create_invoice_inner(&mut conn, sale).expect("sale should succeed");

        // Return 5 units at sale price ₹15.00 each = ₹75.00
        let ret = InvoiceData {
            invoice_type: "sale_return".into(), account_id: cust_id,
            date: "2026-01-16".into(), total: 75.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 5.0, rate: 15.00, discount: 0.0, total: 75.00 }],
            user_id: Some(1),
        };
        create_invoice_inner(&mut conn, ret).expect("sale_return should succeed");

        let je_id = last_je_id(&conn);
        let lines = ledger_lines(&conn, je_id);
        println!("T5 sale_return ledger lines:");
        for (c,p,dr,cr) in &lines { println!("  {} {:?} Dr={} Cr={}", c,p,dr,cr); }

        // Revenue reversal: Dr 4100 / Cr 1100 — this IS posted
        assert!(lines.iter().any(|(c,_,dr,_)| c=="4100" && *dr==7500),
            "Missing Dr 4100 Sales Returns 7500");
        assert!(lines.iter().any(|(c,p,_,cr)| c=="1100" && *p==Some(cust_id) && *cr==7500),
            "Missing Cr 1100 AR 7500 for customer");

        // COGS reversal: Dr 1200 / Cr 5000 at original cost_at_sale (₹10.00 × 5 = ₹50.00 = 5000 paisa)
        // EXPECTED FAILURE: sale_return currently does NOT post COGS reversal
        assert!(lines.iter().any(|(c,_,dr,_)| c=="1200" && *dr==5000),
            "FAIL: Missing Dr 1200 Inventory 5000 — sale_return must reverse COGS at cost_at_sale ₹10.00 × 5");
        assert!(lines.iter().any(|(c,_,_,cr)| c=="5000" && *cr==5000),
            "FAIL: Missing Cr 5000 COGS 5000 — sale_return must reverse COGS");
    }

    // Test 6: Void invoice — not yet implemented
    #[test]
    fn t6_void_invoice_reverses_all_entries() {
        // Void is not implemented. This test documents what's needed:
        // 1. `void_invoice(invoice_id, voided_by, void_reason)` command
        // 2. Sets invoices.voided_at, voided_by, void_reason
        // 3. Posts a reversing journal entry (reverses_journal_entry_id set to original JE)
        // 4. Net of original + reversing in posted_ledger_entries = 0 for all accounts
        // 5. Original rows unchanged (immutability confirmed by reading ledger_entries)
        panic!("EXPECTED FAILURE: void_invoice is not implemented. \
                Implement void_invoice() in invoices.rs, register in lib.rs, then update this test.");
    }

    // Test 7: Payment across two open invoices — AR balance correct; FIFO allocation missing
    #[test]
    fn t7_payment_ar_balance_and_fifo_allocation() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 200, 1000);

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};
        use crate::payments::{create_payment_inner, Payment};

        // Invoice 1: ₹100.00
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-01".into(), total: 100.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 10.00, discount: 0.0, total: 100.00 }],
            user_id: Some(1),
        }).unwrap();
        let inv1_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Invoice 2: ₹150.00
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-05".into(), total: 150.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 15.0, rate: 10.00, discount: 0.0, total: 150.00 }],
            user_id: Some(1),
        }).unwrap();
        let inv2_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        let ar_before = ar_balance(&conn, cust_id);
        assert_eq!(ar_before, 25000, "AR before payment should be 25000 paisa (₹250.00)");

        // Payment of ₹120.00 (covers invoice 1 fully + ₹20 of invoice 2)
        create_payment_inner(&mut conn, Payment {
            id: None, account_id: cust_id, payment_type: "receipt".into(),
            amount: 120.00, tid: "TXN001".into(),
            document_date: "2026-01-10".into(),
            settlement_account_code: "1000".into(),
            user_id: Some(1),
        }).unwrap();
        let pay_id: i64 = conn.query_row("SELECT MAX(id) FROM payments", [], |r| r.get(0)).unwrap();

        let ar_after = ar_balance(&conn, cust_id);
        println!("T7 AR balance after payment={} (expected 13000)", ar_after);
        assert_eq!(ar_after, 13000, "AR after ₹120.00 payment should be 13000 paisa (₹130.00)");

        // FIFO allocation — EXPECTED FAILURE: not implemented
        let alloc_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM payment_allocations WHERE payment_id=?1",
            params![pay_id], |r| r.get(0)
        ).unwrap();
        println!("T7 alloc_count={} (expected 2 for FIFO across 2 invoices)", alloc_count);
        // Suppress inv1_id/inv2_id unused warnings
        let _ = (inv1_id, inv2_id);
        assert_eq!(alloc_count, 2,
            "FAIL: FIFO payment_allocations not implemented — expected 2 rows (inv1 fully, inv2 partial), got 0");
    }

    // Test 8: Backdated invoice — running balance correct at all chronological points
    #[test]
    fn t8_backdated_invoice_balance_correct() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 300, 1000);

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};

        // Post invoice dated 2026-01-10 first (inserted first but later date)
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-10".into(), total: 150.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 15.0, rate: 10.00, discount: 0.0, total: 150.00 }],
            user_id: Some(1),
        }).unwrap();

        // Post backdated invoice dated 2026-01-01 (inserted second but earlier date)
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-01".into(), total: 100.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 10.00, discount: 0.0, total: 100.00 }],
            user_id: Some(1),
        }).unwrap();

        // Balance at 2026-01-05 should be 10000 (only the backdated invoice ≤ 2026-01-05)
        let bal_jan5: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries
             WHERE account_code='1100' AND party_id=?1 AND entry_date <= '2026-01-05'",
            params![cust_id], |r| r.get(0)
        ).unwrap();
        println!("T8 AR balance at 2026-01-05 = {} (expected 10000)", bal_jan5);
        assert_eq!(bal_jan5, 10000, "Balance at 2026-01-05 should be 10000 (only backdated invoice)");

        // Balance at 2026-01-15 should be 25000 (both invoices)
        let bal_jan15: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries
             WHERE account_code='1100' AND party_id=?1 AND entry_date <= '2026-01-15'",
            params![cust_id], |r| r.get(0)
        ).unwrap();
        println!("T8 AR balance at 2026-01-15 = {} (expected 25000)", bal_jan15);
        assert_eq!(bal_jan15, 25000, "Balance at 2026-01-15 should be 25000 (both invoices)");
    }

    // Test 9: Failed transaction leaves no orphan data
    #[test]
    fn t9_failed_transaction_no_orphan_data() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 5, 1000); // only 5 units!

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};

        // Try to sell 10 units (exceeds stock of 5) — should fail
        let payload = InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-15".into(), total: 100.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 10.00, discount: 0.0, total: 100.00 }],
            user_id: Some(1),
        };
        let result = create_invoice_inner(&mut conn, payload);
        println!("T9 error: {:?}", result);
        assert!(result.is_err(), "Should return error for insufficient stock");
        assert!(result.unwrap_err().contains("Insufficient stock"), "Error should mention insufficient stock");

        // No invoice rows
        let inv_count: i64 = conn.query_row("SELECT COUNT(*) FROM invoices", [], |r| r.get(0)).unwrap();
        assert_eq!(inv_count, 0, "No invoice row should exist after failed transaction");

        // No journal entries
        let je_count: i64 = conn.query_row("SELECT COUNT(*) FROM journal_entries", [], |r| r.get(0)).unwrap();
        assert_eq!(je_count, 0, "No journal_entry should exist after failed transaction");

        // No ledger entries
        let le_count: i64 = conn.query_row("SELECT COUNT(*) FROM ledger_entries", [], |r| r.get(0)).unwrap();
        assert_eq!(le_count, 0, "No ledger_entry should exist after failed transaction");

        // Inventory unchanged
        let (qty, val) = inventory_row(&conn, inv_id);
        assert_eq!(qty, 5, "Inventory qty must be unchanged after failed transaction");
        assert_eq!(val, 5000, "Inventory value must be unchanged after failed transaction");
    }
}
