// Temporary audit test — run with:
//   cargo test --test db_audit -- --nocapture
// Delete this file after the audit is complete.

use rusqlite::{Connection, params};

const DB_PATH: &str = r"C:\Users\musaw\AppData\Roaming\com.abd.tauri-app\bloc.db";

fn open_db() -> Connection {
    let conn = Connection::open(DB_PATH).expect("Failed to open bloc.db");
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn
}

#[test]
fn audit_schema_migrations_version() {
    let conn = open_db();
    let mut stmt = conn.prepare(
        "SELECT version, applied_at FROM schema_migrations ORDER BY version"
    ).unwrap();
    let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    println!("\n=== schema_migrations ===");
    for (v, at) in &rows {
        println!("  v{}: applied at {}", v, at);
    }
    let max_version = rows.last().map(|(v, _)| *v).unwrap_or(0);
    println!("  MAX VERSION APPLIED: {}", max_version);
    assert_eq!(max_version, 11, "Expected v11 to be applied");
}

#[test]
fn audit_v11_conversion_expressions() {
    // Verify ROUND was used (not bare CAST) by reading the migration from the source.
    // This test documents what the v11 SQL does — read migrations.rs to confirm.
    // The expressions we verify here are the resulting column types in sqlite_master.
    let conn = open_db();

    println!("\n=== Column types after v11 rebuild ===");

    let tables = ["accounts", "invoices", "invoice_items", "inventory", "payments", "expenses"];
    for table in &tables {
        let sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
            params![table],
            |r| r.get(0),
        ).unwrap_or_else(|_| format!("TABLE {} NOT FOUND", table));
        println!("\n  {}:\n{}", table, sql);
    }
}

#[test]
fn audit_invoice_line_item_consistency() {
    let conn = open_db();

    println!("\n=== Invoice total vs. sum(item totals) consistency ===");

    // For each invoice, check if sum of invoice_items.total == invoices.total
    let mut stmt = conn.prepare(
        "SELECT i.id, i.type, i.total,
                COALESCE(SUM(ii.total), 0) AS items_sum,
                i.discount,
                i.total - i.discount AS net_total
         FROM invoices i
         LEFT JOIN invoice_items ii ON ii.invoice_id = i.id
         WHERE i.voided_at IS NULL
         GROUP BY i.id
         ORDER BY i.id"
    ).unwrap();

    let rows: Vec<(i64, String, i64, i64, i64, i64)> = stmt.query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    }).unwrap().filter_map(|r| r.ok()).collect();

    let mut mismatches = 0i32;
    if rows.is_empty() {
        println!("  (no invoices found — clean database)");
    }
    for (id, inv_type, total, items_sum, discount, _net) in &rows {
        // The invoice.total is the pre-discount total. Items may sum to it or not.
        let diff = total - items_sum;
        let flag = if diff != 0 { " *** MISMATCH ***" } else { "" };
        if diff != 0 { mismatches += 1; }
        println!("  invoice #{} ({}) total={} items_sum={} discount={} diff={}{}",
                 id, inv_type, total, items_sum, discount, diff, flag);
    }
    println!("  MISMATCHES: {}", mismatches);
    assert_eq!(mismatches, 0, "Some invoices have item totals that don't match the invoice total");
}

#[test]
fn audit_sqlite_master_indexes_and_triggers() {
    let conn = open_db();

    println!("\n=== sqlite_master: indexes ===");
    let mut stmt = conn.prepare(
        "SELECT name, tbl_name, sql FROM sqlite_master WHERE type='index' ORDER BY tbl_name, name"
    ).unwrap();
    let rows: Vec<(String, String, Option<String>)> = stmt.query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    }).unwrap().filter_map(|r| r.ok()).collect();
    for (name, tbl, _sql) in &rows {
        println!("  [{tbl}] {name}");
    }

    println!("\n=== sqlite_master: triggers ===");
    let mut stmt2 = conn.prepare(
        "SELECT name, tbl_name, sql FROM sqlite_master WHERE type='trigger' ORDER BY tbl_name, name"
    ).unwrap();
    let triggers: Vec<(String, String, String)> = stmt2.query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    }).unwrap().filter_map(|r| r.ok()).collect();
    for (name, tbl, _sql) in &triggers {
        println!("  [{tbl}] {name}");
    }

    println!("\n=== Specific checks ===");

    // 1. idx_payments_tid (from v2, must survive v11 rebuild)
    let tid_idx: Option<String> = conn.query_row(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_payments_tid'",
        [], |r| r.get(0)
    ).ok();
    println!("  idx_payments_tid: {}", tid_idx.as_deref().unwrap_or("MISSING"));

    // 2. double-void triggers on all rebuilt tables
    let void_triggers = ["prevent_double_void_invoices","prevent_double_void_payments",
                         "prevent_double_void_expenses"];
    for t in &void_triggers {
        let exists: bool = conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type='trigger' AND name=?1",
            params![t], |_| Ok(true)
        ).unwrap_or(false);
        println!("  {}: {}", t, if exists { "EXISTS" } else { "MISSING !!!" });
    }

    // 3. FK integrity check
    println!("\n=== PRAGMA foreign_key_check ===");
    let fk_issues: Vec<(String, i64, String, i64)> = {
        let mut stmt3 = conn.prepare("PRAGMA foreign_key_check").unwrap();
        stmt3.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };
    if fk_issues.is_empty() {
        println!("  OK — no FK violations");
    } else {
        for (table, rowid, parent, fkid) in &fk_issues {
            println!("  FK VIOLATION: table={} rowid={} parent={} fkid={}", table, rowid, parent, fkid);
        }
    }

    // Assertion: idx_payments_tid must exist
    assert!(tid_idx.is_some(), "idx_payments_tid is missing after v11 rebuild");
    // Assertion: all double-void triggers must exist
    for t in &void_triggers {
        let exists: bool = conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type='trigger' AND name=?1",
            params![t], |_| Ok(true)
        ).unwrap_or(false);
        assert!(exists, "Trigger {} is missing after v11 rebuild", t);
    }
    // Assertion: no FK violations
    assert!(fk_issues.is_empty(), "FK violations found: {:?}", fk_issues);
}
