<div align="center">

# 📒 Bloc

**An offline-first desktop accounting app for small businesses: invoicing, inventory, payments and a real double-entry ledger.**

![Status](https://img.shields.io/badge/status-in%20development-yellow?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=black)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat-square&logo=typescript&logoColor=white)
![SQLite](https://img.shields.io/badge/SQLite-003B57?style=flat-square&logo=sqlite&logoColor=white)
![Tailwind](https://img.shields.io/badge/Tailwind%20CSS-4-06B6D4?style=flat-square&logo=tailwindcss&logoColor=white)

</div>

---

## Overview

Bloc is a native desktop app for shop owners who need proper books without a subscription or an internet connection. The Rust backend owns all business logic and talks to a local SQLite database. The React frontend calls it through typed Tauri commands, with no HTTP server in between.

The core is a **double-entry accounting engine**: every invoice, return, payment and void posts balanced journal entries, and the reports are built from the ledger rather than from loose totals.

## Features

- 🧾 **Invoicing:** credit and cash sales, sale returns, voids with full reversal, backdated invoices
- 📦 **Inventory:** stock levels with **weighted-average cost (WAC)**, with cost of goods sold (COGS) captured at the moment of sale
- 💸 **Payments:** receivables with **FIFO allocation** against open invoices and automatic application of advances
- 📚 **Double-entry ledger:** chart of accounts, journal, immutable posted lines, balanced-entry enforcement
- 👥 **Contacts:** customers and suppliers with credit limits and per-party account summaries
- 🧮 **Expenses, fixed assets and depreciation**
- 📊 **Analytics:** income statement and balance sheet built from the ledger
- 🖨️ **Printable invoices**
- 🔐 **Local auth** with bcrypt, plus database **backup and restore**

## Engineering highlights

| | |
|---|---|
| **Money as integers** | All amounts are stored as integer *paisa*, so there is no floating-point drift. Migration v11 rebuilt the legacy `REAL` tables. |
| **Versioned migrations** | Schema evolves through 15 numbered, forward-only migrations (v1 → v15) in `migrations.rs`, applied on startup. |
| **Transactional posting** | Each business event posts inside one SQLite transaction. A failure leaves no orphan rows. |
| **Accounting test suite** | **24 Rust tests** in `journal.rs` cover balanced or unbalanced entries, WAC after later purchases, COGS reversal on returns, cash round-off, FIFO allocation, void rules and full-ledger balance checks. |
| **36 Tauri commands** | Typed IPC surface between the React UI and the Rust core. |

## Tech stack

| Layer | Tech |
|---|---|
| Desktop shell | Tauri 2 |
| Backend | Rust, `rusqlite` (bundled SQLite), `bcrypt`, `serde` |
| Frontend | React 19, TypeScript, Vite, Tailwind CSS 4, shadcn/ui (Radix), Framer Motion, cmdk |

## Getting started

**Prerequisites:** Node.js 18+, the Rust toolchain, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/fast-na-ana/bloc.git
cd bloc
npm install
npm run tauri dev
```

Run the accounting tests:

```bash
cd src-tauri && cargo test
```

Build a release installer:

```bash
npm run tauri build
```

## Project structure

```
src/                     React + TypeScript UI
  components/            one *Screen.tsx per feature, plus shadcn/ui primitives in ui/
  lib/                   invoke() wrappers, invoice maths, formatting
src-tauri/src/           Rust core
  journal.rs             double-entry posting engine + test suite
  invoices.rs            sales, returns, voids, WAC/COGS
  payments.rs            receipts and FIFO allocation
  migrations.rs          versioned schema migrations
  analytics.rs           income statement & balance sheet
docs/design/             early UI mock-up
```

## Roadmap

- [x] Chart of accounts, journal and double-entry posting
- [x] Invoices, returns, voids, payments, inventory costing
- [x] Contacts with credit limits
- [x] Printable invoices
- [ ] One-click PDF export
- [ ] Packaged Windows installer release
- [ ] Multi-user roles

> A related experiment, [**Bloc Books**](https://github.com/fast-na-ana/bloc-books), explores the same product on top of the open-source Frappe Books codebase.

## Contributors

- Abdullah Saeed, [@fast-na-ana](https://github.com/fast-na-ana)
- Hania Zahra
