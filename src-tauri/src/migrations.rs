use rusqlite::{Connection, Result};

// Forward-only numbered migrations. Each entry = one version.
// Version number = index + 1. Never edit a committed entry; add a new one.
const MIGRATIONS: &[&str] = &[
    // v1 — baseline schema
    "
    CREATE TABLE IF NOT EXISTS users (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        username      TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        role          TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS accounts (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        name         TEXT NOT NULL,
        type         TEXT NOT NULL,
        credit_limit REAL    DEFAULT 0,
        created_at   DATETIME DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS inventory (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        name        TEXT NOT NULL,
        stock_level REAL DEFAULT 0,
        cost_price  REAL DEFAULT 0,
        sale_price  REAL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS invoices (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        type       TEXT NOT NULL,
        account_id INTEGER NOT NULL,
        date       DATETIME NOT NULL,
        total      REAL    DEFAULT 0,
        discount   REAL    DEFAULT 0,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY(account_id) REFERENCES accounts(id)
    );
    CREATE TABLE IF NOT EXISTS invoice_items (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        invoice_id   INTEGER NOT NULL,
        inventory_id INTEGER NOT NULL,
        quantity     REAL NOT NULL,
        rate         REAL NOT NULL,
        discount     REAL DEFAULT 0,
        total        REAL NOT NULL,
        FOREIGN KEY(invoice_id)   REFERENCES invoices(id),
        FOREIGN KEY(inventory_id) REFERENCES inventory(id)
    );
    CREATE TABLE IF NOT EXISTS payments (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        account_id     INTEGER NOT NULL,
        type           TEXT NOT NULL,
        amount         REAL NOT NULL,
        tid            TEXT UNIQUE,
        document_date  DATETIME,
        bank_cash_name TEXT,
        FOREIGN KEY(account_id) REFERENCES accounts(id)
    );
    CREATE TABLE IF NOT EXISTS expenses (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        description TEXT NOT NULL,
        amount      REAL NOT NULL,
        date        DATETIME NOT NULL
    );
    CREATE TABLE IF NOT EXISTS reminders (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        account_id INTEGER NOT NULL,
        note       TEXT NOT NULL,
        due_date   DATETIME NOT NULL,
        status     TEXT NOT NULL,
        FOREIGN KEY(account_id) REFERENCES accounts(id)
    );
    CREATE TABLE IF NOT EXISTS assets (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        name              TEXT NOT NULL,
        value             REAL NOT NULL,
        depreciation_rate REAL DEFAULT 0
    );
    ",

    // v2 — enforce unique TID on existing databases
    // (new databases already have UNIQUE on the column from v1;
    //  existing databases get an equivalent unique index instead)
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_payments_tid ON payments(tid);",

    // v3 — chart of accounts with 16 preset rows
    "
    CREATE TABLE IF NOT EXISTS accounts_chart (
        code           TEXT PRIMARY KEY,
        name           TEXT NOT NULL,
        account_type   TEXT NOT NULL CHECK(account_type IN ('asset','liability','equity','revenue','expense')),
        normal_balance TEXT NOT NULL CHECK(normal_balance IN ('debit','credit'))
    );
    INSERT OR IGNORE INTO accounts_chart (code, name, account_type, normal_balance) VALUES
        ('1000','Cash / Bank','asset','debit'),
        ('1100','Accounts Receivable','asset','debit'),
        ('1200','Inventory','asset','debit'),
        ('1300','Fixed Assets','asset','debit'),
        ('1310','Accumulated Depreciation','asset','credit'),
        ('1900','Supplier Advances','asset','debit'),
        ('2000','Accounts Payable','liability','credit'),
        ('2200','Customer Advances','liability','credit'),
        ('3000','Retained Earnings','equity','credit'),
        ('4000','Sales Revenue','revenue','credit'),
        ('4100','Sales Returns','revenue','debit'),
        ('5000','Cost of Goods Sold','expense','debit'),
        ('5100','Purchase Returns','expense','credit'),
        ('6000','Operating Expenses','expense','debit'),
        ('6100','Depreciation Expense','expense','debit'),
        ('9000','Rounding Difference','expense','debit');
    ",

    // v4 — WAC fields on inventory
    "
    ALTER TABLE inventory ADD COLUMN qty_on_hand INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE inventory ADD COLUMN total_value_paisa INTEGER NOT NULL DEFAULT 0;
    UPDATE inventory SET
        qty_on_hand = CAST(ROUND(stock_level) AS INTEGER),
        total_value_paisa = CAST(ROUND(stock_level * cost_price * 100) AS INTEGER);
    ",

    // v5 — cost_at_sale on invoice_items
    "ALTER TABLE invoice_items ADD COLUMN cost_at_sale INTEGER;",

    // v6 — void metadata + double-void triggers
    "
    ALTER TABLE invoices ADD COLUMN voided_at DATETIME;
    ALTER TABLE invoices ADD COLUMN voided_by INTEGER REFERENCES users(id);
    ALTER TABLE invoices ADD COLUMN void_reason TEXT;
    ALTER TABLE payments ADD COLUMN voided_at DATETIME;
    ALTER TABLE payments ADD COLUMN voided_by INTEGER REFERENCES users(id);
    ALTER TABLE payments ADD COLUMN void_reason TEXT;
    ALTER TABLE expenses ADD COLUMN voided_at DATETIME;
    ALTER TABLE expenses ADD COLUMN voided_by INTEGER REFERENCES users(id);
    ALTER TABLE expenses ADD COLUMN void_reason TEXT;
    CREATE TRIGGER prevent_double_void_invoices
    BEFORE UPDATE OF voided_at ON invoices WHEN OLD.voided_at IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'Invoice is already voided'); END;
    CREATE TRIGGER prevent_double_void_payments
    BEFORE UPDATE OF voided_at ON payments WHEN OLD.voided_at IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'Payment is already voided'); END;
    CREATE TRIGGER prevent_double_void_expenses
    BEFORE UPDATE OF voided_at ON expenses WHEN OLD.voided_at IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'Expense is already voided'); END;
    ",

    // v7 — fixed_assets table (replaces old assets for new installs; migrate existing rows)
    "
    CREATE TABLE IF NOT EXISTS fixed_assets (
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        name                TEXT NOT NULL,
        acquisition_date    DATE NOT NULL,
        original_cost       INTEGER NOT NULL,
        salvage_value       INTEGER NOT NULL DEFAULT 0,
        useful_life_years   REAL NOT NULL,
        method              TEXT NOT NULL DEFAULT 'straight_line' CHECK(method IN ('straight_line','declining_balance')),
        depreciation_rate   REAL NOT NULL,
        voided_at           DATETIME,
        voided_by           INTEGER REFERENCES users(id),
        void_reason         TEXT
    );
    CREATE TRIGGER IF NOT EXISTS prevent_double_void_fixed_assets
    BEFORE UPDATE OF voided_at ON fixed_assets WHEN OLD.voided_at IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'Fixed asset is already voided'); END;
    INSERT OR IGNORE INTO fixed_assets (name, acquisition_date, original_cost, salvage_value, useful_life_years, method, depreciation_rate)
    SELECT name, date('now'), CAST(ROUND(value * 100) AS INTEGER), 0,
        CASE WHEN depreciation_rate > 0 THEN 100.0 / depreciation_rate ELSE 10.0 END,
        'straight_line', depreciation_rate FROM assets;
    ",

    // v8 — payment_allocations
    "
    CREATE TABLE IF NOT EXISTS payment_allocations (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        payment_id   INTEGER NOT NULL REFERENCES payments(id),
        invoice_id   INTEGER NOT NULL REFERENCES invoices(id),
        amount       INTEGER NOT NULL CHECK(amount > 0),
        allocated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        UNIQUE(payment_id, invoice_id)
    );
    CREATE INDEX IF NOT EXISTS idx_alloc_payment ON payment_allocations(payment_id);
    CREATE INDEX IF NOT EXISTS idx_alloc_invoice ON payment_allocations(invoice_id);
    ",

    // v9 — journal_entries header
    "
    CREATE TABLE IF NOT EXISTS journal_entries (
        id                        INTEGER PRIMARY KEY AUTOINCREMENT,
        entry_date                DATE NOT NULL,
        posted_at                 DATETIME DEFAULT CURRENT_TIMESTAMP,
        description               TEXT NOT NULL,
        reference_type            TEXT NOT NULL CHECK(reference_type IN ('invoice','payment','expense','depreciation','void','opening','adjustment')),
        reference_id              INTEGER,
        reverses_journal_entry_id INTEGER REFERENCES journal_entries(id),
        created_by                INTEGER NOT NULL REFERENCES users(id),
        is_posted                 INTEGER NOT NULL DEFAULT 0 CHECK(is_posted IN (0,1))
    );
    CREATE INDEX IF NOT EXISTS idx_jentry_ref      ON journal_entries(reference_type, reference_id);
    CREATE INDEX IF NOT EXISTS idx_jentry_date     ON journal_entries(entry_date);
    CREATE INDEX IF NOT EXISTS idx_jentry_reverses ON journal_entries(reverses_journal_entry_id)
        WHERE reverses_journal_entry_id IS NOT NULL;
    ",

    // v10 — ledger_entries, triggers, view
    "
    CREATE TABLE IF NOT EXISTS ledger_entries (
        id               INTEGER PRIMARY KEY AUTOINCREMENT,
        journal_entry_id INTEGER NOT NULL REFERENCES journal_entries(id),
        account_code     TEXT    NOT NULL REFERENCES accounts_chart(code),
        party_id         INTEGER REFERENCES accounts(id),
        debit            INTEGER NOT NULL DEFAULT 0 CHECK(debit  >= 0),
        credit           INTEGER NOT NULL DEFAULT 0 CHECK(credit >= 0),
        CHECK((debit > 0 AND credit = 0) OR (credit > 0 AND debit = 0))
    );
    CREATE INDEX IF NOT EXISTS idx_ledger_entry   ON ledger_entries(journal_entry_id);
    CREATE INDEX IF NOT EXISTS idx_ledger_account ON ledger_entries(account_code);
    CREATE INDEX IF NOT EXISTS idx_ledger_party   ON ledger_entries(party_id) WHERE party_id IS NOT NULL;
    CREATE TRIGGER enforce_journal_balance
    BEFORE UPDATE OF is_posted ON journal_entries
    WHEN NEW.is_posted = 1 AND OLD.is_posted = 0
    BEGIN
        SELECT CASE
            WHEN COALESCE((SELECT SUM(debit)  FROM ledger_entries WHERE journal_entry_id = OLD.id), 0) !=
                 COALESCE((SELECT SUM(credit) FROM ledger_entries WHERE journal_entry_id = OLD.id), 0)
            THEN RAISE(ABORT, 'Journal entry does not balance: total debits != total credits')
        END;
    END;
    CREATE TRIGGER prevent_lines_on_posted_entry
    BEFORE INSERT ON ledger_entries
    WHEN (SELECT is_posted FROM journal_entries WHERE id = NEW.journal_entry_id) = 1
    BEGIN SELECT RAISE(ABORT, 'Cannot add lines to a posted journal entry'); END;
    CREATE TRIGGER prevent_double_post
    BEFORE UPDATE OF is_posted ON journal_entries WHEN OLD.is_posted = 1
    BEGIN SELECT RAISE(ABORT, 'Journal entry is already posted'); END;
    CREATE TRIGGER immutable_ledger_update
    BEFORE UPDATE ON ledger_entries
    WHEN (SELECT is_posted FROM journal_entries WHERE id = OLD.journal_entry_id) = 1
    BEGIN SELECT RAISE(ABORT, 'Cannot modify a posted ledger entry — post a reversing entry instead'); END;
    CREATE TRIGGER immutable_ledger_delete
    BEFORE DELETE ON ledger_entries
    WHEN (SELECT is_posted FROM journal_entries WHERE id = OLD.journal_entry_id) = 1
    BEGIN SELECT RAISE(ABORT, 'Cannot delete a posted ledger entry — post a reversing entry instead'); END;
    CREATE VIEW posted_ledger_entries AS
    SELECT le.*, je.entry_date, je.description AS je_description, je.reference_type, je.reference_id
    FROM ledger_entries le
    JOIN journal_entries je ON je.id = le.journal_entry_id
    WHERE je.is_posted = 1;
    ",

    // v11 — REAL → INTEGER table rebuilds (all money columns converted to paisa)
    "
    PRAGMA foreign_keys = OFF;
    BEGIN;
    CREATE TABLE accounts_new (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        name         TEXT NOT NULL,
        type         TEXT NOT NULL,
        credit_limit INTEGER NOT NULL DEFAULT 0,
        created_at   DATETIME DEFAULT CURRENT_TIMESTAMP
    );
    INSERT INTO accounts_new SELECT id, name, type, CAST(ROUND(credit_limit * 100) AS INTEGER), created_at FROM accounts;
    DROP TABLE accounts;
    ALTER TABLE accounts_new RENAME TO accounts;
    CREATE TABLE invoices_new (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        type        TEXT NOT NULL,
        account_id  INTEGER NOT NULL,
        date        DATETIME NOT NULL,
        total       INTEGER NOT NULL DEFAULT 0,
        discount    INTEGER NOT NULL DEFAULT 0,
        created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
        voided_at   DATETIME,
        voided_by   INTEGER REFERENCES users(id),
        void_reason TEXT,
        FOREIGN KEY(account_id) REFERENCES accounts(id)
    );
    INSERT INTO invoices_new SELECT id, type, account_id, date, CAST(ROUND(total * 100) AS INTEGER), CAST(ROUND(discount * 100) AS INTEGER), created_at, voided_at, voided_by, void_reason FROM invoices;
    DROP TABLE invoices;
    ALTER TABLE invoices_new RENAME TO invoices;
    CREATE TRIGGER prevent_double_void_invoices
    BEFORE UPDATE OF voided_at ON invoices WHEN OLD.voided_at IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'Invoice is already voided'); END;
    CREATE TABLE invoice_items_new (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        invoice_id   INTEGER NOT NULL,
        inventory_id INTEGER NOT NULL,
        quantity     REAL NOT NULL,
        rate         INTEGER NOT NULL DEFAULT 0,
        discount     INTEGER NOT NULL DEFAULT 0,
        total        INTEGER NOT NULL DEFAULT 0,
        cost_at_sale INTEGER,
        FOREIGN KEY(invoice_id)   REFERENCES invoices(id),
        FOREIGN KEY(inventory_id) REFERENCES inventory(id)
    );
    INSERT INTO invoice_items_new SELECT id, invoice_id, inventory_id, quantity, CAST(ROUND(rate * 100) AS INTEGER), CAST(ROUND(discount * 100) AS INTEGER), CAST(ROUND(total * 100) AS INTEGER), cost_at_sale FROM invoice_items;
    DROP TABLE invoice_items;
    ALTER TABLE invoice_items_new RENAME TO invoice_items;
    CREATE TABLE inventory_new (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        name              TEXT NOT NULL,
        qty_on_hand       INTEGER NOT NULL DEFAULT 0,
        total_value_paisa INTEGER NOT NULL DEFAULT 0,
        cost_price        INTEGER NOT NULL DEFAULT 0,
        sale_price        INTEGER NOT NULL DEFAULT 0
    );
    INSERT INTO inventory_new SELECT id, name, qty_on_hand, total_value_paisa, CAST(ROUND(cost_price * 100) AS INTEGER), CAST(ROUND(sale_price * 100) AS INTEGER) FROM inventory;
    DROP TABLE inventory;
    ALTER TABLE inventory_new RENAME TO inventory;
    CREATE TABLE payments_new (
        id                      INTEGER PRIMARY KEY AUTOINCREMENT,
        account_id              INTEGER NOT NULL,
        type                    TEXT NOT NULL,
        amount                  INTEGER NOT NULL DEFAULT 0,
        tid                     TEXT UNIQUE,
        document_date           DATETIME,
        settlement_account_code TEXT NOT NULL DEFAULT '1000' REFERENCES accounts_chart(code),
        voided_at               DATETIME,
        voided_by               INTEGER REFERENCES users(id),
        void_reason             TEXT,
        FOREIGN KEY(account_id) REFERENCES accounts(id)
    );
    INSERT INTO payments_new (id, account_id, type, amount, tid, document_date, settlement_account_code, voided_at, voided_by, void_reason)
        SELECT id, account_id, type, CAST(ROUND(amount * 100) AS INTEGER), tid, document_date, '1000', voided_at, voided_by, void_reason FROM payments;
    DROP TABLE payments;
    ALTER TABLE payments_new RENAME TO payments;
    CREATE UNIQUE INDEX IF NOT EXISTS idx_payments_tid ON payments(tid);
    CREATE TRIGGER prevent_double_void_payments
    BEFORE UPDATE OF voided_at ON payments WHEN OLD.voided_at IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'Payment is already voided'); END;
    CREATE TABLE expenses_new (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        description TEXT NOT NULL,
        amount      INTEGER NOT NULL DEFAULT 0,
        date        DATETIME NOT NULL,
        voided_at   DATETIME,
        voided_by   INTEGER REFERENCES users(id),
        void_reason TEXT
    );
    INSERT INTO expenses_new SELECT id, description, CAST(ROUND(amount * 100) AS INTEGER), date, voided_at, voided_by, void_reason FROM expenses;
    DROP TABLE expenses;
    ALTER TABLE expenses_new RENAME TO expenses;
    CREATE TRIGGER prevent_double_void_expenses
    BEFORE UPDATE OF voided_at ON expenses WHEN OLD.voided_at IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'Expense is already voided'); END;
    COMMIT;
    PRAGMA foreign_keys = ON;
    ",
];

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version > current_version {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                rusqlite::params![version],
            )?;
        }
    }

    Ok(())
}
