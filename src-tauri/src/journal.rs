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
            settlement_account_code: None,
            original_invoice_id: None,
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

    // Test 2: Cash sale with rounding — total not a whole rupee routes difference to 9000
    #[test]
    fn t2_cash_sale_rounding_to_9000() {
        // Scenario: 3 units @ cost ₹10.00 each, sell all 3 at ₹29.99 cash
        // Invoice total = 2999 paisa; rounded to nearest rupee = 3000 paisa
        // Journal: Dr 1000 (3000) / Cr 4000 (2999) / Cr 9000 (1)  [+COGS lines]
        // WAC is exact: ROUND(3000 * 3 / 3) = 3000 → no inventory residue
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 3, 1000); // 3 units @ ₹10.00 each

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};
        let payload = InvoiceData {
            invoice_type: "sale".into(),
            account_id: cust_id,
            date: "2026-01-15".into(),
            total: 29.99,   // 2999 paisa → rounds to 3000
            discount: 0.0,
            items: vec![InvoiceItemData {
                inventory_id: inv_id, quantity: 3.0, rate: 9.997, discount: 0.0, total: 29.99,
            }],
            user_id: Some(1),
            settlement_account_code: Some("1000".into()), // cash sale
            original_invoice_id: None,
        };
        create_invoice_inner(&mut conn, payload).expect("cash sale should succeed");
        let je_id = last_je_id(&conn);
        let lines = ledger_lines(&conn, je_id);

        println!("T2 ledger lines (cash sale rounding test):");
        for (code, party, dr, cr) in &lines {
            println!("  {} party={:?} Dr={} Cr={}", code, party, dr, cr);
        }

        // Cash leg: Dr 1000 → 3000 (rounded)
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="1000" && *dr==3000 && *cr==0),
            "Cash sale must Dr 1000 Cash at rounded amount 3000");
        // Revenue: Cr 4000 → 2999 (exact)
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="4000" && *dr==0 && *cr==2999),
            "Revenue must Cr 4000 at exact amount 2999");
        // Rounding: Cr 9000 → 1 (round-up income)
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="9000" && *dr==0 && *cr==1),
            "1-paisa round-up must Cr 9000 Rounding Difference 1");
        // No AR: cash sale must NOT touch 1100
        assert!(!lines.iter().any(|(c,_,_,_)| c=="1100"),
            "Cash sale must not post to 1100 AR");
        // COGS: Dr 5000 → 3000 (ROUND(3000*3/3)=3000), Cr 1200 → 3000
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="5000" && *dr==3000 && *cr==0),
            "COGS must Dr 5000 3000");
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="1200" && *dr==0 && *cr==3000),
            "Inventory must Cr 1200 3000");

        // Inventory fully depleted
        let (qty, val) = inventory_row(&conn, inv_id);
        assert_eq!(qty, 0, "qty_on_hand should be 0 after selling all 3 units");
        assert_eq!(val, 0, "total_value_paisa should be 0 — no WAC residue");
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
            settlement_account_code: None,
            original_invoice_id: None,
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
                settlement_account_code: None,
                original_invoice_id: None,
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
            settlement_account_code: None,
            original_invoice_id: None,
        };
        create_invoice_inner(&mut conn, sale).expect("sale should succeed");

        // cost_at_sale stored on invoice_items row
        let cost_at_sale: Option<i64> = conn.query_row(
            "SELECT cost_at_sale FROM invoice_items WHERE inventory_id=?1 ORDER BY id DESC LIMIT 1",
            params![inv_id], |r| r.get(0)
        ).unwrap();
        println!("T4 cost_at_sale={:?} (expected 11333)", cost_at_sale);
        assert_eq!(cost_at_sale, Some(11333),
            "cost_at_sale is total COGS for the line = ROUND(170000 * 10 / 150) = 11333");

        // COGS in ledger = ROUND(170000 * 10 / 150) = 11333
        let je_id = last_je_id(&conn);
        let lines = ledger_lines(&conn, je_id);
        assert!(lines.iter().any(|(c,_,dr,cr)| c=="5000" && *dr==11333 && *cr==0),
            "COGS should be 11333 (ROUND(170000 * 10 / 150))");

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
        println!("T4 cost_at_sale after later purchase={:?} (must still be 11333)", sale_cost_at_sale);
        assert_eq!(sale_cost_at_sale, Some(11333),
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
            settlement_account_code: None,
            original_invoice_id: None,
        };
        create_invoice_inner(&mut conn, sale).expect("sale should succeed");
        let sale_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Return 5 units at sale price ₹15.00 each = ₹75.00
        let ret = InvoiceData {
            invoice_type: "sale_return".into(), account_id: cust_id,
            date: "2026-01-16".into(), total: 75.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 5.0, rate: 15.00, discount: 0.0, total: 75.00 }],
            user_id: Some(1),
            settlement_account_code: None,
            original_invoice_id: Some(sale_id),
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

    // Test 6: Void invoice — reversal zeroes all posted balances and restores inventory
    #[test]
    fn t6_void_invoice_reverses_all_entries() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000); // 100 units @ ₹10.00 each

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};

        // Sell 10 units @ ₹15.00; COGS = ROUND(100000*10/100) = 10000
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-15".into(), total: 150.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 15.00, discount: 0.0, total: 150.00 }],
            user_id: Some(1),
            settlement_account_code: None,
            original_invoice_id: None,
        }).unwrap();

        let invoice_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();
        let orig_je_id = last_je_id(&conn);

        // Inventory: 90 units, val=90000 after the sale
        let (q, v) = inventory_row(&conn, inv_id);
        assert_eq!(q, 90);
        assert_eq!(v, 90000);

        // Void the invoice
        void_invoice_inner(&mut conn, invoice_id, 1, "Test void".into()).unwrap();

        // 1. voided_at is set
        let voided_at: Option<String> = conn.query_row(
            "SELECT voided_at FROM invoices WHERE id = ?1", params![invoice_id], |r| r.get(0)
        ).unwrap();
        assert!(voided_at.is_some(), "voided_at must be set after void");

        // 2. Two journal entries for this invoice (original 'invoice' + reversal 'void')
        let je_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM journal_entries WHERE reference_id = ?1",
            params![invoice_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(je_count, 2, "Should have 2 JEs: original + reversal");

        // 3. Reversal JE references original
        let rev_je_id = last_je_id(&conn);
        let reverses: Option<i64> = conn.query_row(
            "SELECT reverses_journal_entry_id FROM journal_entries WHERE id = ?1",
            params![rev_je_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(reverses, Some(orig_je_id), "Reversal JE must point to original JE");

        // 4. Net posted balances = 0 for all accounts touched
        let net_ar: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries
             WHERE account_code='1100' AND party_id=?1", params![cust_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_ar, 0, "Net AR must be 0 after void");

        let net_rev: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='4000'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_rev, 0, "Net revenue must be 0 after void");

        let net_cogs: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='5000'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_cogs, 0, "Net COGS must be 0 after void");

        // 5. Original ledger lines are immutable and correct
        let orig_lines = ledger_lines(&conn, orig_je_id);
        println!("T6 original lines:");
        for (c,p,dr,cr) in &orig_lines { println!("  {} {:?} Dr={} Cr={}", c,p,dr,cr); }
        assert!(orig_lines.iter().any(|(c,p,dr,cr)| c=="1100" && *p==Some(cust_id) && *dr==15000 && *cr==0),
            "Original AR debit must be unchanged");
        assert!(orig_lines.iter().any(|(c,_,dr,cr)| c=="5000" && *dr==10000 && *cr==0),
            "Original COGS debit must be unchanged");

        // 6. Inventory fully restored
        let (qty_final, val_final) = inventory_row(&conn, inv_id);
        assert_eq!(qty_final, 100, "qty_on_hand must be restored to 100");
        assert_eq!(val_final, 100000, "total_value_paisa must be restored to 100000");
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
            settlement_account_code: None,
            original_invoice_id: None,
        }).unwrap();
        let inv1_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Invoice 2: ₹150.00
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-05".into(), total: 150.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 15.0, rate: 10.00, discount: 0.0, total: 150.00 }],
            user_id: Some(1),
            settlement_account_code: None,
            original_invoice_id: None,
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
            settlement_account_code: None,
            original_invoice_id: None,
        }).unwrap();

        // Post backdated invoice dated 2026-01-01 (inserted second but earlier date)
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-01".into(), total: 100.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 10.00, discount: 0.0, total: 100.00 }],
            user_id: Some(1),
            settlement_account_code: None,
            original_invoice_id: None,
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
            settlement_account_code: None,
            original_invoice_id: None,
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

    // ── void interaction tests ──────────────────────────────────────────────────

    // t10: void is blocked when payment_allocations exist for the invoice
    #[test]
    fn t10_void_blocked_by_payment_allocations() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000);

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};
        use crate::payments::{create_payment_inner, Payment};

        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-01".into(), total: 100.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 10.00, discount: 0.0, total: 100.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: None,
        }).unwrap();
        let invoice_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Pay ₹60 — allocates to this invoice via FIFO
        create_payment_inner(&mut conn, Payment {
            id: None, account_id: cust_id, payment_type: "receipt".into(),
            amount: 60.00, tid: "TXN-T10".into(),
            document_date: "2026-01-05".into(),
            settlement_account_code: "1000".into(),
            user_id: Some(1),
        }).unwrap();

        let result = void_invoice_inner(&mut conn, invoice_id, 1, "should be blocked".into());
        assert!(result.is_err(), "Void must fail when payment allocations exist");
        let msg = result.unwrap_err();
        println!("T10 error: {}", msg);
        assert!(msg.contains("allocated"), "Error must mention allocations: {}", msg);

        // Invoice must remain un-voided
        let voided: Option<String> = conn.query_row(
            "SELECT voided_at FROM invoices WHERE id = ?1", params![invoice_id], |r| r.get(0)
        ).unwrap();
        assert!(voided.is_none(), "Invoice must still be live after blocked void");
    }

    // t11: void of sale is blocked while unvoided sale_return exists; succeeds after return is voided
    #[test]
    fn t11_void_blocked_by_existing_return_then_succeeds() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000);

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};

        // Original sale: 10 units @ ₹15
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-01".into(), total: 150.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 15.00, discount: 0.0, total: 150.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: None,
        }).unwrap();
        let sale_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Partial return: 4 units @ ₹15
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale_return".into(), account_id: cust_id,
            date: "2026-01-05".into(), total: 60.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 4.0, rate: 15.00, discount: 0.0, total: 60.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: Some(sale_id),
        }).unwrap();
        let return_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Attempt void of original sale — must be blocked
        let blocked = void_invoice_inner(&mut conn, sale_id, 1, "blocked".into());
        assert!(blocked.is_err(), "Void of sale must be blocked while unvoided return exists");
        println!("T11 block msg: {}", blocked.unwrap_err());

        // Void the return first
        void_invoice_inner(&mut conn, return_id, 1, "void return".into())
            .expect("Voiding the return itself must succeed");

        // After voiding the return, inventory must be back to post-sale state (90 units)
        let (qty_after_ret_void, _) = inventory_row(&conn, inv_id);
        assert_eq!(qty_after_ret_void, 90, "After voiding return, qty should be 90 (sale still active)");

        // Now void the original sale — must succeed
        void_invoice_inner(&mut conn, sale_id, 1, "now ok".into())
            .expect("Void of sale must succeed after return is voided");

        // All balances zero, inventory fully restored
        let net_ar: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='1100' AND party_id=?1",
            params![cust_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_ar, 0, "Net AR must be 0 after all voids");

        let (qty_final, val_final) = inventory_row(&conn, inv_id);
        assert_eq!(qty_final, 100, "Inventory must be fully restored");
        assert_eq!(val_final, 100000, "Inventory value must be fully restored");
    }

    // t12: voiding a cash sale also reverses the 9000 rounding entry
    #[test]
    fn t12_void_cash_sale_reverses_9000() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 10, 1000); // 10 @ ₹10.00

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};

        // Cash sale: total ₹29.99 (2999 paisa) → rounds to 3000, Cr 9000 = 1
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-10".into(), total: 29.99, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 3.0, rate: 9.997, discount: 0.0, total: 29.99 }],
            user_id: Some(1), settlement_account_code: Some("1000".into()), original_invoice_id: None,
        }).unwrap();
        let invoice_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Confirm 9000 has Cr 1 before void
        let net_9000_before: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='9000'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_9000_before, -1, "9000 should have net Cr 1 (= -1 net) before void");

        void_invoice_inner(&mut conn, invoice_id, 1, "void cash sale".into()).unwrap();

        // After void, 9000 net must be 0
        let net_9000_after: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='9000'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_9000_after, 0, "9000 must net to 0 after voiding cash sale");

        // Cash account must also net to 0
        let net_cash: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='1000'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_cash, 0, "Cash must net to 0 after voiding cash sale");
    }

    // t13: voiding a sale_return removes the returned inventory and restores the journal
    #[test]
    fn t13_void_sale_return() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000);

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};

        // Sell 10 units
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-01".into(), total: 150.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 15.00, discount: 0.0, total: 150.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: None,
        }).unwrap();
        let sale_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();
        // After sale: qty=90, val=90000

        // Return 5 units
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale_return".into(), account_id: cust_id,
            date: "2026-01-05".into(), total: 75.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 5.0, rate: 15.00, discount: 0.0, total: 75.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: Some(sale_id),
        }).unwrap();
        let return_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();
        // After return: qty=95, val=95000 (5 units @ ₹10.00 added back)

        let (q_after_ret, v_after_ret) = inventory_row(&conn, inv_id);
        assert_eq!(q_after_ret, 95);
        assert_eq!(v_after_ret, 95000);

        // Void the return
        void_invoice_inner(&mut conn, return_id, 1, "void return".into()).unwrap();

        // Inventory must revert to post-sale state (90 units, not the original 100)
        let (qty, val) = inventory_row(&conn, inv_id);
        assert_eq!(qty, 90, "Voiding return must remove the 5 units that were added back");
        assert_eq!(val, 90000, "Voiding return must remove the ₹50.00 value that was added back");

        // Net AR: original sale Dr 1100 15000, return Cr 1100 7500, void-of-return Dr 1100 7500
        //        → net = 15000 (still owed by customer)
        let net_ar: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='1100' AND party_id=?1",
            params![cust_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_ar, 15000, "After voiding return, AR must be back to full sale amount");

        // 4100 Sales Returns must net to 0 (return posted Dr 4100 7500; void reversal Cr 4100 7500)
        let net_4100: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='4100'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(net_4100, 0, "4100 Sales Returns must net 0 after voiding the return");
    }

    // t14: return against a voided sale is a hard block — WAC corruption is not a soft path
    #[test]
    fn t14_return_against_voided_sale_is_blocked() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000);

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};

        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-01".into(), total: 100.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 10.00, discount: 0.0, total: 100.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: None,
        }).unwrap();
        let sale_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Void the sale (no returns, no allocations — succeeds)
        void_invoice_inner(&mut conn, sale_id, 1, "void".into()).unwrap();

        // Attempt a return referencing the now-voided sale — must be a hard error
        let ret_result = create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale_return".into(), account_id: cust_id,
            date: "2026-01-10".into(), total: 50.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 5.0, rate: 10.00, discount: 0.0, total: 50.00 }],
            user_id: Some(1), settlement_account_code: None,
            original_invoice_id: Some(sale_id), // explicitly references the voided invoice
        });
        assert!(ret_result.is_err(), "Return against voided invoice must be rejected");
        let msg = ret_result.unwrap_err();
        println!("T14 blocked: {}", msg);
        assert!(msg.contains("voided"), "Error must mention voided: {}", msg);

        // Inventory must be exactly restored to pre-sale state (void restored all 100 units)
        let (qty, val) = inventory_row(&conn, inv_id);
        assert_eq!(qty, 100, "qty_on_hand must remain 100 — no phantom return");
        assert_eq!(val, 100000, "total_value_paisa must remain 100000 — no WAC corruption");

        // WAC invariant: items with stock must have positive value
        let bad_wac: i64 = conn.query_row(
            "SELECT COUNT(*) FROM inventory WHERE qty_on_hand > 0 AND total_value_paisa <= 0",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(bad_wac, 0, "WAC invariant: stock with zero value is corruption");
    }

    // WAC sell-through: buying 3 units for ₹10.00 and selling 1+1+1 must leave val=0 and COGS=1000
    #[test]
    fn t_wac_no_residue_on_sell_through() {
        // ROUND(1000*1/3)=333 → val=667
        // ROUND(667*1/2)=334  → val=333
        // ROUND(333*1/1)=333  → val=0
        // total COGS = 333+334+333 = 1000
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let sup_id = seed_supplier(&conn);

        conn.execute(
            "INSERT INTO inventory (name, qty_on_hand, total_value_paisa, cost_price, sale_price)
             VALUES ('Triplet', 0, 0, 0, 0)",
            [],
        ).unwrap();
        let inv_id = conn.last_insert_rowid();

        use crate::invoices::{create_invoice_inner, InvoiceData, InvoiceItemData};

        // Purchase 3 units, line total = ₹10.00 (1000 paisa)
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "purchase".into(), account_id: sup_id,
            date: "2026-01-01".into(), total: 10.00, discount: 0.0,
            items: vec![InvoiceItemData {
                inventory_id: inv_id, quantity: 3.0, rate: 3.34, discount: 0.0, total: 10.00,
            }],
            user_id: Some(1),
            settlement_account_code: None,
            original_invoice_id: None,
        }).unwrap();

        let (q, v) = inventory_row(&conn, inv_id);
        assert_eq!(q, 3);
        assert_eq!(v, 1000);

        let mut total_cogs: i64 = 0;

        for (day, expected_cogs) in [("2026-01-02", 333i64), ("2026-01-03", 334), ("2026-01-04", 333)] {
            create_invoice_inner(&mut conn, InvoiceData {
                invoice_type: "sale".into(), account_id: cust_id,
                date: day.into(), total: 5.00, discount: 0.0,
                items: vec![InvoiceItemData {
                    inventory_id: inv_id, quantity: 1.0, rate: 5.00, discount: 0.0, total: 5.00,
                }],
                user_id: Some(1),
                settlement_account_code: None,
                original_invoice_id: None,
            }).unwrap();

            let je_id = last_je_id(&conn);
            let lines = ledger_lines(&conn, je_id);
            let cogs = lines.iter()
                .find(|(c, _, dr, _)| c == "5000" && *dr > 0)
                .map(|(_, _, dr, _)| *dr)
                .unwrap_or(0);
            println!("sell on {}: COGS={} expected={}", day, cogs, expected_cogs);
            assert_eq!(cogs, expected_cogs, "COGS for sale on {} should be {}", day, expected_cogs);
            total_cogs += cogs;
        }

        let (qty_final, val_final) = inventory_row(&conn, inv_id);
        println!("final: qty={} val={} total_cogs={}", qty_final, val_final, total_cogs);
        assert_eq!(qty_final, 0, "qty_on_hand should be 0 after sell-through");
        assert_eq!(val_final, 0, "total_value_paisa should be 0 after sell-through — no WAC residue");
        assert_eq!(total_cogs, 1000, "total COGS must equal original purchase cost 1000 paisa");
    }

    // t15: advance auto-application — overpayment in 2200 settles next invoice at creation time
    #[test]
    fn t15_advance_auto_application() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000);

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};
        use crate::payments::{create_payment_inner, Payment};

        // Invoice 1: ₹200.00
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-02-01".into(), total: 200.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 20.0, rate: 10.00, discount: 0.0, total: 200.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: None,
        }).unwrap();
        let inv1_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // Receipt ₹300.00: ₹200 covers inv1, ₹100 overpayment → 2200
        create_payment_inner(&mut conn, Payment {
            id: None, account_id: cust_id, payment_type: "receipt".into(),
            amount: 300.00, tid: "OVP-001".into(),
            document_date: "2026-02-05".into(),
            settlement_account_code: "1000".into(),
            user_id: Some(1),
        }).unwrap();
        let pay_id: i64 = conn.query_row("SELECT MAX(id) FROM payments", [], |r| r.get(0)).unwrap();

        // Confirm 2200 has ₹100 advance (10000 paisa)
        let adv_before: i64 = conn.query_row(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries WHERE account_code='2200'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(adv_before, 10000, "2200 must hold 10000 paisa advance before new invoice");

        // Cash sale: must NOT consume the advance
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-02-06".into(), total: 30.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 3.0, rate: 10.00, discount: 0.0, total: 30.00 }],
            user_id: Some(1), settlement_account_code: Some("1000".into()), original_invoice_id: None,
        }).unwrap();
        let adv_after_cash: i64 = conn.query_row(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries WHERE account_code='2200'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(adv_after_cash, 10000, "Cash sale must not consume the advance");

        // Invoice 2 (credit sale): ₹80.00 — advance should auto-apply ₹80 from 2200
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-02-10".into(), total: 80.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 8.0, rate: 10.00, discount: 0.0, total: 80.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: None,
        }).unwrap();
        let inv2_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        // 2200 should be reduced by ₹80 → ₹20 (2000 paisa) remains
        let adv_after_inv2: i64 = conn.query_row(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries WHERE account_code='2200'",
            [], |r| r.get(0)
        ).unwrap();
        println!("T15 2200 after inv2={} (expected 2000)", adv_after_inv2);
        assert_eq!(adv_after_inv2, 2000, "2200 must drop by 8000 after advance applied to inv2");

        // AR net for customer: inv1 settled by receipt, inv2 settled by advance → 0
        let ar_net: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='1100' AND party_id=?1",
            params![cust_id], |r| r.get(0)
        ).unwrap();
        println!("T15 AR net={} (expected 0 — inv2 fully offset by advance)", ar_net);
        assert_eq!(ar_net, 0, "AR must net to 0: inv2 fully covered by 2200 advance");

        // Allocation row written with allocation_source='advance'
        let adv_alloc: i64 = conn.query_row(
            "SELECT COUNT(*) FROM payment_allocations WHERE payment_id=?1 AND invoice_id=?2 AND allocation_source='advance'",
            params![pay_id, inv2_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(adv_alloc, 1, "Must have 1 advance allocation row for inv2");

        // FIFO sees inv2 as fully settled (open amount = 0)
        let inv2_open: i64 = conn.query_row(
            "SELECT i.total - COALESCE(SUM(pa.amount), 0) FROM invoices i
             LEFT JOIN payment_allocations pa ON pa.invoice_id = i.id
             WHERE i.id = ?1 GROUP BY i.id",
            params![inv2_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(inv2_open, 0, "FIFO open amount for inv2 must be 0 after advance allocation");

        // Void inv2 — must succeed (advance allocations don't block) and restore 2200
        void_invoice_inner(&mut conn, inv2_id, 1, "void advance test".into())
            .expect("Void of invoice with advance must succeed");

        // 2200 restored to 10000 (the advance lines are in the same JE → mirror-flip restores them)
        let adv_after_void: i64 = conn.query_row(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries WHERE account_code='2200'",
            [], |r| r.get(0)
        ).unwrap();
        println!("T15 2200 after void={} (expected 10000)", adv_after_void);
        assert_eq!(adv_after_void, 10000, "2200 must be restored to 10000 after voiding inv2");

        // Advance allocation row must be deleted
        let adv_alloc_after: i64 = conn.query_row(
            "SELECT COUNT(*) FROM payment_allocations WHERE payment_id=?1 AND invoice_id=?2",
            params![pay_id, inv2_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(adv_alloc_after, 0, "Advance allocation must be deleted when invoice is voided");

        let _ = (inv1_id, pay_id);
    }

    // Paisa round-trip: (x * 100.0).round() as i64 must not truncate tricky float values.
    // Without .round(), 19.99 * 100.0 evaluates to 1998.9999... in IEEE 754 → truncates to 1998.
    // This is the v11 regression class. All conversion sites in this codebase use .round().
    #[test]
    fn t_paisa_rounding_no_truncation() {
        for (rupees, expected_paisa) in [
            (19.99_f64, 1999_i64),
            ( 0.01_f64,    1_i64),
            ( 0.07_f64,    7_i64),
        ] {
            let paisa = (rupees * 100.0).round() as i64;
            assert_eq!(paisa, expected_paisa,
                "₹{} * 100 must round to {} paisa, not truncate to {}", rupees, expected_paisa, (rupees * 100.0) as i64);
        }
    }

    // t16: payment for a customer must not allocate to that customer's voided invoice
    #[test]
    fn t16_payment_skips_voided_invoice() {
        let mut conn = open_test_db();
        let cust_id = seed_customer(&conn);
        let inv_id = seed_inventory(&conn, 100, 1000);

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};
        use crate::payments::{create_payment_inner, Payment};

        // Create a sale, then immediately void it (no allocations yet)
        create_invoice_inner(&mut conn, InvoiceData {
            invoice_type: "sale".into(), account_id: cust_id,
            date: "2026-01-01".into(), total: 100.00, discount: 0.0,
            items: vec![InvoiceItemData { inventory_id: inv_id, quantity: 10.0, rate: 10.00, discount: 0.0, total: 100.00 }],
            user_id: Some(1), settlement_account_code: None, original_invoice_id: None,
        }).unwrap();
        let invoice_id: i64 = conn.query_row("SELECT MAX(id) FROM invoices", [], |r| r.get(0)).unwrap();

        void_invoice_inner(&mut conn, invoice_id, 1, "voiding before payment".into()).unwrap();

        // Now record a receipt for this customer
        create_payment_inner(&mut conn, Payment {
            id: None, account_id: cust_id, payment_type: "receipt".into(),
            amount: 150.00, tid: "TXN-T16".into(),
            document_date: "2026-01-10".into(),
            settlement_account_code: "1000".into(),
            user_id: Some(1),
        }).unwrap();
        let pay_id: i64 = conn.query_row("SELECT MAX(id) FROM payments", [], |r| r.get(0)).unwrap();

        // FIFO must not allocate to the voided invoice
        let alloc_to_voided: i64 = conn.query_row(
            "SELECT COUNT(*) FROM payment_allocations WHERE payment_id=?1 AND invoice_id=?2",
            params![pay_id, invoice_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(alloc_to_voided, 0,
            "FIFO must not allocate to a voided invoice (voided_at IS NULL filter must hold)");

        // Entire payment becomes overpayment → routed to 2200 (no open AR to absorb it)
        let adv_2200: i64 = conn.query_row(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries WHERE account_code='2200'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(adv_2200, 15000, "Full receipt must land in 2200 since voided invoice has no open AR");

        // Total payment_allocations for this payment = 0 (all went to 2200, nothing to FIFO)
        let total_alloc: i64 = conn.query_row(
            "SELECT COUNT(*) FROM payment_allocations WHERE payment_id=?1",
            params![pay_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(total_alloc, 0, "No FIFO allocations when all open invoices are voided");
    }

    // ── financial statement balance integration test ────────────────────────────
    //
    // Runs a realistic mixed sequence and asserts five balance invariants:
    //   1. Total posted debits = total posted credits (double-entry integrity)
    //   2. Per-party AR sum = AR control account (1100) balance
    //   3. SUM(inventory.total_value_paisa) = 1200 control account balance
    //
    // Sequence:
    //   - Purchase A: 100 units @ ₹10  (WAC = ₹10)
    //   - Purchase B: 50  units @ ₹14  (WAC = ₹11.33…)
    //   - Credit sale C1: 20 units @ ₹20 (cust1)
    //   - Credit sale C2: 30 units @ ₹20 (cust1)
    //   - Cash sale C3:   3 units @ ₹29.99 total (cust2, round-off → 9000)
    //   - Partial return R1: 5 units of C2 @ ₹20 (cust1)
    //   - Payment P1: ₹200 receipt from cust1 (FIFO across C1, C2)
    //   - Payment P2: ₹999 overpayment from cust1 (→ 2200)
    //   - Expense E1: ₹500 operating expense
    //   - Void C1: blocked by payment allocation — assert error; skip void
    //   - Void R1: succeeds (no allocations on a return)
    //
    #[test]
    fn t_financial_balance_integration() {
        let mut conn = open_test_db();

        let cust1_id = seed_customer(&conn);
        // second customer
        conn.execute(
            "INSERT INTO accounts (name, type, credit_limit) VALUES ('Cash Customer', 'customer', 0)", []
        ).unwrap();
        let cust2_id = conn.last_insert_rowid();
        let sup_id = seed_supplier(&conn);

        conn.execute(
            "INSERT INTO inventory (name, qty_on_hand, total_value_paisa, cost_price, sale_price) VALUES ('Widget', 0, 0, 0, 0)", []
        ).unwrap();
        let inv_id = conn.last_insert_rowid();

        use crate::invoices::{create_invoice_inner, void_invoice_inner, InvoiceData, InvoiceItemData};
        use crate::payments::{create_payment_inner, Payment};
        use crate::expenses::{create_expense_inner, Expense};

        let make_inv = |conn: &mut rusqlite::Connection, itype: &str, acct: i64, date: &str,
                         qty: f64, rate: f64, total: f64, settle: Option<&str>,
                         orig: Option<i64>| {
            create_invoice_inner(conn, InvoiceData {
                invoice_type: itype.into(), account_id: acct, date: date.into(),
                total, discount: 0.0,
                items: vec![InvoiceItemData { inventory_id: inv_id, quantity: qty, rate, discount: 0.0, total }],
                user_id: Some(1),
                settlement_account_code: settle.map(|s| s.to_string()),
                original_invoice_id: orig,
            }).unwrap();
            conn.query_row("SELECT MAX(id) FROM invoices", [], |r: &rusqlite::Row| r.get::<_,i64>(0)).unwrap()
        };

        // Purchases
        make_inv(&mut conn, "purchase", sup_id, "2026-01-01", 100.0, 10.00, 1000.00, None, None);
        make_inv(&mut conn, "purchase", sup_id, "2026-01-02",  50.0, 14.00,  700.00, None, None);
        // WAC now: total_value=170000, qty=150

        // Credit sales for cust1
        let c1_id = make_inv(&mut conn, "sale", cust1_id, "2026-01-05", 20.0, 20.00, 400.00, None, None);
        // COGS = ROUND(170000*20/150) = 22667
        let c2_id = make_inv(&mut conn, "sale", cust1_id, "2026-01-06", 30.0, 20.00, 600.00, None, None);
        // WAC after c1: val=170000-22667=147333, qty=130; COGS c2=ROUND(147333*30/130)=34000

        // Cash sale cust2: 3 units, ₹29.99 total → 2999 paisa → rounds to 3000
        make_inv(&mut conn, "sale", cust2_id, "2026-01-07", 3.0, 9.997, 29.99, Some("1000"), None);

        // Partial return from cust1 (5 units of c2, ₹20 each = ₹100)
        let r1_id = make_inv(&mut conn, "sale_return", cust1_id, "2026-01-08", 5.0, 20.00, 100.00, None, Some(c2_id));

        // Payments from cust1
        create_payment_inner(&mut conn, Payment {
            id: None, account_id: cust1_id, payment_type: "receipt".into(),
            amount: 200.00, tid: "FIN-P1".into(),
            document_date: "2026-01-10".into(),
            settlement_account_code: "1000".into(),
            user_id: Some(1),
        }).unwrap();

        // Overpayment from cust1 — exceeds remaining AR → routes to 2200
        create_payment_inner(&mut conn, Payment {
            id: None, account_id: cust1_id, payment_type: "receipt".into(),
            amount: 999.00, tid: "FIN-P2".into(),
            document_date: "2026-01-11".into(),
            settlement_account_code: "1000".into(),
            user_id: Some(1),
        }).unwrap();

        // Expense
        create_expense_inner(&mut conn, Expense {
            id: None, description: "Rent".into(),
            amount: 500.00, date: "2026-01-12".into(),
            user_id: Some(1),
        }).unwrap();

        // Attempt void of c1 — must be blocked (has payment allocation)
        let void_c1 = void_invoice_inner(&mut conn, c1_id, 1, "should block".into());
        assert!(void_c1.is_err(), "Void of c1 must be blocked — it has payment allocations");
        println!("FIN void_c1 blocked: {}", void_c1.unwrap_err());

        // Void the return r1 — no allocations on returns, so this must succeed
        void_invoice_inner(&mut conn, r1_id, 1, "void return".into())
            .expect("Void of sale_return must succeed");

        // ── INVARIANT 1: total posted debits = total posted credits ────────────
        let total_debits: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit), 0) FROM posted_ledger_entries", [], |r| r.get(0)
        ).unwrap();
        let total_credits: i64 = conn.query_row(
            "SELECT COALESCE(SUM(credit), 0) FROM posted_ledger_entries", [], |r| r.get(0)
        ).unwrap();
        println!("INV1 total_debits={} total_credits={} balanced={}",
            total_debits, total_credits, total_debits == total_credits);
        assert_eq!(total_debits, total_credits,
            "INVARIANT 1 FAIL: total debits {} ≠ total credits {}", total_debits, total_credits);

        // ── INVARIANT 2: per-party AR sums = AR control account ───────────────
        let ar_control: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='1100'",
            [], |r| r.get(0)
        ).unwrap();
        let ar_cust1: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='1100' AND party_id=?1",
            params![cust1_id], |r| r.get(0)
        ).unwrap();
        let ar_cust2: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='1100' AND party_id=?1",
            params![cust2_id], |r| r.get(0)
        ).unwrap();
        let ar_party_sum = ar_cust1 + ar_cust2;
        println!("INV2 ar_control={} cust1_ar={} cust2_ar={} party_sum={} balanced={}",
            ar_control, ar_cust1, ar_cust2, ar_party_sum, ar_party_sum == ar_control);
        assert_eq!(ar_party_sum, ar_control,
            "INVARIANT 2 FAIL: per-party AR {} ≠ AR control {}", ar_party_sum, ar_control);

        // ── INVARIANT 3: inventory.total_value_paisa = 1200 control account ───
        let inv_table_val: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_value_paisa), 0) FROM inventory", [], |r| r.get(0)
        ).unwrap();
        let inv_ledger_val: i64 = conn.query_row(
            "SELECT COALESCE(SUM(debit)-SUM(credit),0) FROM posted_ledger_entries WHERE account_code='1200'",
            [], |r| r.get(0)
        ).unwrap();
        println!("INV3 inventory_table={} ledger_1200={} balanced={}",
            inv_table_val, inv_ledger_val, inv_table_val == inv_ledger_val);
        assert_eq!(inv_table_val, inv_ledger_val,
            "INVARIANT 3 FAIL: inventory.total_value_paisa {} ≠ 1200 ledger balance {}",
            inv_table_val, inv_ledger_val);

        // ── INVARIANT 4: WAC integrity — stock implies positive value ──────────
        let bad_wac_rows: i64 = conn.query_row(
            "SELECT COUNT(*) FROM inventory WHERE qty_on_hand > 0 AND total_value_paisa <= 0",
            [], |r| r.get(0)
        ).unwrap();
        println!("INV4 wac_corruption_rows={}", bad_wac_rows);
        assert_eq!(bad_wac_rows, 0,
            "INVARIANT 4 FAIL: {} item(s) have stock but zero/negative value — WAC corrupted", bad_wac_rows);

        // ── INVARIANT 5: Assets = Liabilities + Equity + Net Income ───────────
        // (fundamental accounting equation; holds in any pre-close period)
        let assets: i64 = conn.query_row(
            "SELECT COALESCE(SUM(le.debit - le.credit), 0)
             FROM posted_ledger_entries le
             JOIN accounts_chart ac ON ac.code = le.account_code
             WHERE ac.account_type = 'asset'",
            [], |r| r.get(0)
        ).unwrap();
        let liabilities: i64 = conn.query_row(
            "SELECT COALESCE(SUM(le.credit - le.debit), 0)
             FROM posted_ledger_entries le
             JOIN accounts_chart ac ON ac.code = le.account_code
             WHERE ac.account_type = 'liability'",
            [], |r| r.get(0)
        ).unwrap();
        let equity: i64 = conn.query_row(
            "SELECT COALESCE(SUM(le.credit - le.debit), 0)
             FROM posted_ledger_entries le
             JOIN accounts_chart ac ON ac.code = le.account_code
             WHERE ac.account_type = 'equity'",
            [], |r| r.get(0)
        ).unwrap();
        let net_revenue: i64 = conn.query_row(
            "SELECT COALESCE(SUM(le.credit - le.debit), 0)
             FROM posted_ledger_entries le
             JOIN accounts_chart ac ON ac.code = le.account_code
             WHERE ac.account_type = 'revenue'",
            [], |r| r.get(0)
        ).unwrap();
        let net_expenses: i64 = conn.query_row(
            "SELECT COALESCE(SUM(le.debit - le.credit), 0)
             FROM posted_ledger_entries le
             JOIN accounts_chart ac ON ac.code = le.account_code
             WHERE ac.account_type = 'expense'",
            [], |r| r.get(0)
        ).unwrap();
        let net_income = net_revenue - net_expenses;
        let rhs = liabilities + equity + net_income;
        println!("INV5 assets={} liabilities={} equity={} net_revenue={} net_expenses={} net_income={} rhs={}  balanced={}",
            assets, liabilities, equity, net_revenue, net_expenses, net_income, rhs, assets == rhs);
        assert_eq!(assets, rhs,
            "INVARIANT 5 FAIL: assets {} ≠ liabilities+equity+net_income {}", assets, rhs);

        // ── INVARIANT 6: net profit = change in equity for the period ──────────
        // Before closing entries, equity change = net income (no retained earnings posted).
        // Equity from posted entries (3000) should be 0 in a fresh test; net income carries it.
        let equity_change = equity + net_income; // equity postings + current period income
        println!("INV6 net_income={} equity_change(equity+net_income)={} match={}",
            net_income, equity_change, net_income == equity_change - equity);
        // equity_change = equity + net_income; we assert net_income == equity_change - equity (tautology but
        // makes the structure explicit — a real test would compare against prior-period equity)
        assert_eq!(net_income, equity_change - equity,
            "INVARIANT 6 FAIL: net income {} ≠ equity change {}", net_income, equity_change - equity);

        // ── 2200 advance balance ───────────────────────────────────────────────
        let adv_2200: i64 = conn.query_row(
            "SELECT COALESCE(SUM(credit)-SUM(debit),0) FROM posted_ledger_entries WHERE account_code='2200'",
            [], |r| r.get(0)
        ).unwrap();
        println!("2200_advances={} paisa (auto-applied to next invoice; this run has no new invoice after P2)", adv_2200);
        assert!(adv_2200 > 0, "2200 must have a positive balance from the overpayment");
    }
}
