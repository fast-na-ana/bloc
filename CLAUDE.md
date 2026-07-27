# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

**Bloc** is a desktop business operations app built with Tauri (Rust backend) + React/TypeScript frontend. It manages invoicing, inventory, payments, expenses, ledger, and analytics — all backed by a local SQLite database.

## Development Commands

```bash
# Start frontend dev server (Vite, port 1420)
npm run dev

# Run full Tauri app in development (frontend + Rust backend)
npm run tauri dev

# Build frontend only
npm run build

# Build production desktop binary
npm run tauri build

# Type-check frontend
npx tsc --noEmit
```

> Rust backend is compiled by Tauri automatically during `tauri dev` / `tauri build`. To work on Rust independently, `cd src-tauri && cargo check`.

## Architecture

### Two-Process Model

The app has two distinct layers that communicate via Tauri IPC:

- **Frontend** (`src/`) — React 19 + TypeScript, Vite, Tailwind CSS 4, Framer Motion, shadcn/ui
- **Backend** (`src-tauri/src/`) — Rust, SQLite via `rusqlite`, bcrypt auth

Frontend calls backend via `invoke()` from `@tauri-apps/api/core`. There is no HTTP API — all data access goes through named Tauri commands.

### Frontend Structure

- `src/App.tsx` — root component; owns `currentView` state and renders the active screen
- `src/components/Layout.tsx` — sidebar nav + header wrapper used by all authenticated screens
- `src/components/*Screen.tsx` — one file per major feature (Dashboard, Invoice, Payment, Inventory, Expense, Ledger, Analytics, Reminder)
- `src/components/ui/` — shadcn/ui primitives (Button, Card, Input, etc.)
- `src/lib/db.ts` — thin wrappers around `invoke()` for backup/restore
- `src/lib/invoice-calc.ts` — invoice line-item calculation utilities

Path alias `@/` maps to `src/`.

### Backend Structure (`src-tauri/src/`)

| File | Responsibility |
|------|---------------|
| `lib.rs` | Tauri app setup; registers all `#[tauri::command]` handlers |
| `db.rs` | SQLite schema creation & migrations |
| `auth.rs` | Login, admin setup, bcrypt hashing |
| `invoices.rs` | Invoice + invoice_items CRUD, inventory updates |
| `payments.rs` | Payment CRUD |
| `expenses.rs` | Expense CRUD |
| `ledger.rs` | Account/credit-limit management |
| `analytics.rs` | Aggregation queries (income statement, balance sheet) |
| `backup.rs` | Database backup/restore to user-chosen path |
| `assets.rs` | Asset depreciation tracking |

### Database

SQLite file stored in the OS app-data directory (`bloc.db`). Schema is initialized in `db.rs` using `CREATE TABLE IF NOT EXISTS`. Key tables: `users`, `accounts`, `inventory`, `invoices`, `invoice_items`, `payments`, `expenses`, `reminders`, `assets`.

### UI Conventions

- Glassmorphism aesthetic: frosted glass panels, animated floating blob background (defined in `src/index.css`)
- Framer Motion `AnimatePresence` + `motion.div` for page transitions
- All color/theme tokens use Tailwind utility classes; custom CSS variables are in `index.css`
- shadcn/ui components live in `src/components/ui/` — do not hand-write primitive UI elements

## Key Patterns

**Adding a new Tauri command:**
1. Implement the function in the relevant `src-tauri/src/*.rs` file
2. Annotate with `#[tauri::command]`
3. Register it in `lib.rs` inside `.invoke_handler(tauri::generate_handler![...])`
4. Call it from the frontend with `invoke("command_name", { ...args })`

**Adding a new screen:**
1. Create `src/components/FooScreen.tsx`
2. Add the view name to the `currentView` union type in `App.tsx`
3. Render `<FooScreen />` in the view switch in `App.tsx`
4. Add a nav entry in `Layout.tsx`
