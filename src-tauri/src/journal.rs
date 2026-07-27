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
}
