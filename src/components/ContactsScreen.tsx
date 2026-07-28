import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { Input } from "@/components/ui/input";
import { Users, Search, AlertTriangle } from "lucide-react";
import { EmptyState } from "./ui/EmptyState";
import { PartyDetail, Account } from "./PartyDetail";
import { fmt } from "@/lib/format";

interface AccountSummary {
  account_id: number;
  outstanding: number;
}

interface ContactsScreenProps {
  currentUser?: { id: number; username: string; role: string };
}

const containerVars = {
  hidden: { opacity: 0 },
  show: { opacity: 1, transition: { staggerChildren: 0.05 } },
} as any;

const itemVars = {
  hidden: { opacity: 0, y: 15 },
  show: { opacity: 1, y: 0, transition: { duration: 0.4, ease: "easeOut" } },
} as any;

export function ContactsScreen({ currentUser }: ContactsScreenProps) {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [summaries, setSummaries] = useState<Map<number, number>>(new Map());
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [search, setSearch] = useState("");

  const loadData = async () => {
    try {
      const [accs, sums] = await Promise.all([
        invoke<Account[]>("get_accounts"),
        invoke<AccountSummary[]>("get_account_summaries"),
      ]);
      setAccounts(accs);
      setSummaries(new Map(sums.map((s) => [s.account_id, s.outstanding])));
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const filtered = accounts.filter((a) =>
    a.name.toLowerCase().includes(search.toLowerCase()) ||
    a.account_type.toLowerCase().includes(search.toLowerCase())
  );

  const selectedAccount = accounts.find((a) => a.id === selectedId) ?? null;

  const isOverLimit = (acc: Account) => {
    const outstanding = summaries.get(acc.id) ?? 0;
    return acc.credit_limit > 0 && outstanding > acc.credit_limit;
  };

  return (
    <motion.div
      variants={containerVars}
      initial="hidden"
      animate="show"
      className="grid grid-cols-12 gap-6 h-full auto-rows-max"
    >
      {/* Header */}
      <motion.div
        variants={itemVars}
        className="col-span-12 glass-widget p-6 hover-lift flex flex-col md:flex-row md:items-center justify-between gap-4"
      >
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-2xl bg-blue-50 border border-blue-100 flex items-center justify-center shadow-xs">
            <Users className="text-blue-600 h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-extrabold text-slate-900 leading-tight">
              Customers & Suppliers
            </h1>
            <p className="text-slate-500 text-sm font-medium">
              Party ledgers, credit limits, and payback analytics
            </p>
          </div>
        </div>
      </motion.div>

      {/* Left: contact list */}
      <motion.div
        variants={itemVars}
        className="col-span-12 lg:col-span-4 glass-widget p-4 hover-lift flex flex-col gap-3"
      >
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400 pointer-events-none" />
          <Input
            className="pl-9 bg-white/80 border-slate-200 text-slate-900 font-semibold"
            placeholder="Search contacts..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>

        {filtered.length === 0 ? (
          <EmptyState icon={Users} title="No Contacts" description="No accounts match your search." />
        ) : (
          <ul className="flex flex-col gap-1 overflow-y-auto max-h-[calc(100vh-260px)]">
            {filtered.map((acc) => {
              const outstanding = summaries.get(acc.id) ?? 0;
              const overLimit = isOverLimit(acc);
              const isActive = acc.id === selectedId;
              return (
                <li key={acc.id}>
                  <button
                    onClick={() => setSelectedId(acc.id)}
                    className={`w-full text-left px-4 py-3 rounded-xl transition-all duration-200 flex items-center justify-between gap-2 cursor-pointer ${
                      isActive
                        ? "bg-blue-600 text-white shadow-md shadow-blue-600/25"
                        : "hover:bg-blue-50/70 text-slate-800"
                    }`}
                  >
                    <div className="min-w-0">
                      <div className="flex items-center gap-1.5">
                        <span className="font-bold text-sm truncate">{acc.name}</span>
                        {overLimit && (
                          <AlertTriangle className={`h-3.5 w-3.5 shrink-0 ${isActive ? "text-yellow-300" : "text-rose-500"}`} />
                        )}
                      </div>
                      <span className={`text-xs font-semibold uppercase tracking-wide ${isActive ? "text-blue-100" : "text-slate-500"}`}>
                        {acc.account_type}
                      </span>
                    </div>
                    <div className="text-right shrink-0">
                      <div className={`text-sm font-extrabold ${isActive ? "text-white" : outstanding > 0 ? "text-blue-700" : "text-slate-400"}`}>
                        {outstanding !== 0 ? fmt(outstanding) : "—"}
                      </div>
                      {overLimit && (
                        <div className={`text-[10px] font-bold uppercase ${isActive ? "text-yellow-300" : "text-rose-500"}`}>
                          Over Limit
                        </div>
                      )}
                    </div>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </motion.div>

      {/* Right: detail panel */}
      <motion.div
        variants={itemVars}
        className="col-span-12 lg:col-span-8 glass-widget p-6 hover-lift"
      >
        {selectedAccount ? (
          <PartyDetail
            account={selectedAccount}
            currentUser={currentUser}
            onLimitUpdated={loadData}
          />
        ) : (
          <div className="flex items-center justify-center h-full min-h-[300px]">
            <EmptyState
              icon={Users}
              title="Select a Contact"
              description="Choose a customer or supplier from the list to view their ledger and credit details."
            />
          </div>
        )}
      </motion.div>
    </motion.div>
  );
}
