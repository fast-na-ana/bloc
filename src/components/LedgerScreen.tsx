import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ShieldAlert, Activity, CreditCard, Users, Receipt } from "lucide-react";
import { EmptyState } from "./ui/EmptyState";

interface Account {
  id: number;
  name: string;
  account_type: string;
  credit_limit: number;
}

interface LedgerEntry {
  date: string;
  description: string;
  debit: number;
  credit: number;
  balance: number;
}

interface PaybackMetrics {
  average_days: number;
  late_payments: number;
}

interface LedgerScreenProps {
  currentUser?: { id: number; username: string; role: string };
}

const containerVars = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.05 }
  }
} as any;

const itemVars = {
  hidden: { opacity: 0, y: 15 },
  show: { opacity: 1, y: 0, transition: { duration: 0.4, ease: "easeOut" } }
} as any;

export function LedgerScreen({ currentUser }: LedgerScreenProps) {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [role, setRole] = useState<"customer" | "supplier">("customer");
  
  const [ledger, setLedger] = useState<LedgerEntry[]>([]);
  const [metrics, setMetrics] = useState<PaybackMetrics | null>(null);

  const [limitPassword, setLimitPassword] = useState("");
  const [newLimit, setNewLimit] = useState<number | "">("");
  const [isModalOpen, setIsModalOpen] = useState(false);

  const loadData = async () => {
    try {
      const acc = await invoke<Account[]>("get_accounts");
      setAccounts(acc);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  useEffect(() => {
    if (selectedAccountId) {
      const id = parseInt(selectedAccountId);
      invoke<LedgerEntry[]>("get_account_ledger", { accountId: id, role })
        .then(setLedger).catch(console.error);

      if (role === "customer") {
        invoke<PaybackMetrics>("analyze_payback_behavior", { accountId: id })
          .then(setMetrics).catch(console.error);
      } else {
        setMetrics(null);
      }
    } else {
      setLedger([]);
      setMetrics(null);
    }
  }, [selectedAccountId, role]);

  const selectedAccount = accounts.find(a => a.id === parseInt(selectedAccountId));

  const handleUpdateLimit = async () => {
    if (!selectedAccount || newLimit === "") return;
    try {
      if (!currentUser || currentUser.role !== "admin") throw new Error("Only Admins can change credit limits.");

      const authCheck: any = await invoke("login", { username: currentUser.username, passwordRaw: limitPassword });
      if (!authCheck.success) throw new Error("Invalid Administrator Password.");

      await invoke("update_credit_limit", {
        accountId: selectedAccount.id,
        newLimit: Number(newLimit),
        currentRole: currentUser?.role ?? "",
      });
      alert("Credit Limit successfully updated!");
      setIsModalOpen(false);
      setLimitPassword("");
      setNewLimit("");
      loadData();
    } catch (err) {
      alert(err);
    }
  };

  return (
    <motion.div 
      variants={containerVars}
      initial="hidden"
      animate="show"
      className="grid grid-cols-12 gap-6 h-full auto-rows-max"
    >
      {/* HEADER BAR */}
      <motion.div variants={itemVars} className="col-span-12 glass-widget p-6 hover-lift flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-2xl bg-blue-50 border border-blue-100 flex items-center justify-center shadow-xs">
            <CreditCard className="text-blue-600 h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-extrabold text-slate-900 leading-tight">
              Party Ledgers & Credit Limits
            </h1>
            <p className="text-slate-500 text-sm font-medium">View chronologically sorted running balances and credit analytics</p>
          </div>
        </div>

        <div className="w-full md:w-80">
          <Select value={selectedAccountId} onValueChange={setSelectedAccountId}>
            <SelectTrigger className="bg-white/80 border-slate-200 focus:ring-blue-500 text-slate-900 font-semibold">
              <SelectValue placeholder="Select Account / Contact..." />
            </SelectTrigger>
            <SelectContent>
              {accounts.map(acc => (
                <SelectItem key={acc.id} value={acc.id.toString()}>
                  {acc.name} ({acc.account_type})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </motion.div>

      {/* ACCOUNT DETAILS & LEDGER PANEL */}
      {selectedAccount ? (
        <motion.div variants={itemVars} className="col-span-12 glass-widget p-6 hover-lift space-y-6">
          <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4 border-b border-slate-200 pb-4">
            <div>
              <h2 className="text-xl font-bold text-slate-900 flex items-center gap-2">
                <Users className="h-5 w-5 text-blue-600" />
                {selectedAccount.name}
              </h2>
              <p className="text-xs font-semibold text-slate-500 uppercase tracking-wider mt-1">
                Account Type: <span className="text-blue-600">{selectedAccount.account_type}</span>
              </p>
            </div>

            {/* Credit Metrics Section */}
            <div className="flex flex-wrap items-center gap-4">
              {metrics && (
                <div className="flex items-center gap-4 text-xs font-semibold bg-slate-50/80 px-4 py-2 rounded-xl border border-slate-200">
                  <div className="flex items-center gap-1.5 text-slate-700">
                    <Activity className="h-4 w-4 text-blue-600" />
                    <span>Avg Pay-Back: <strong className={metrics.average_days > 30 ? "text-rose-600" : "text-slate-900"}>{metrics.average_days} Days</strong></span>
                  </div>
                  {metrics.late_payments > 0 && (
                    <div className="flex items-center gap-1.5 text-rose-600 font-bold">
                      <ShieldAlert className="h-4 w-4" />
                      <span>{metrics.late_payments} Late Payment(s)</span>
                    </div>
                  )}
                </div>
              )}

              <div className="text-xs font-semibold bg-slate-50/80 px-4 py-2 rounded-xl border border-slate-200 flex items-center gap-2">
                <span className="text-slate-600">Credit Limit:</span>
                <strong className="text-slate-900 font-extrabold text-sm">{selectedAccount.credit_limit > 0 ? `$${selectedAccount.credit_limit.toFixed(2)}` : "Unlimited"}</strong>
                {currentUser?.role === "admin" && (
                  <Dialog open={isModalOpen} onOpenChange={setIsModalOpen}>
                    <DialogTrigger asChild>
                      <Button variant="link" size="sm" className="h-auto p-0 text-blue-600 font-bold hover:text-blue-800 ml-1">Change</Button>
                    </DialogTrigger>
                    <DialogContent className="glass-widget bg-white/95 border-slate-200 backdrop-blur-xl">
                      <DialogHeader>
                        <DialogTitle className="text-lg font-extrabold text-slate-900">Update Credit Limit</DialogTitle>
                      </DialogHeader>
                      <div className="space-y-4 pt-2">
                        <div className="space-y-1.5">
                          <label className="text-xs font-bold uppercase tracking-wider text-slate-600">New Credit Limit ($)</label>
                          <Input type="number" value={newLimit} onChange={(e) => setNewLimit(parseFloat(e.target.value) || "")} className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
                        </div>
                        <div className="space-y-1.5 border-t border-slate-200 pt-4">
                          <label className="text-xs font-bold uppercase tracking-wider text-rose-600 flex items-center gap-1.5">
                            <ShieldAlert className="h-4 w-4" /> Verify Admin Password ({currentUser?.username})
                          </label>
                          <Input type="password" value={limitPassword} onChange={(e) => setLimitPassword(e.target.value)} placeholder="Enter Admin Password" className="bg-white/80 border-rose-200 text-slate-900 font-semibold" />
                        </div>
                        <Button className="w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold shadow-md shadow-blue-600/20" onClick={handleUpdateLimit}>Authorize & Save</Button>
                      </div>
                    </DialogContent>
                  </Dialog>
                )}
              </div>
            </div>
          </div>

          <Tabs value={role} onValueChange={(v) => setRole(v as "customer" | "supplier")} className="w-full">
            <TabsList className="bg-slate-100/80 border border-slate-200/80 p-1 rounded-xl mb-4">
              <TabsTrigger value="customer" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Customer Ledger</TabsTrigger>
              <TabsTrigger value="supplier" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Supplier Ledger</TabsTrigger>
            </TabsList>
            <TabsContent value="customer" className="mt-0">
              <LedgerTable entries={ledger} role="customer" />
            </TabsContent>
            <TabsContent value="supplier" className="mt-0">
              <LedgerTable entries={ledger} role="supplier" />
            </TabsContent>
          </Tabs>
        </motion.div>
      ) : (
        <motion.div variants={itemVars} className="col-span-12 glass-widget p-12 hover-lift text-center">
          <EmptyState icon={Users} title="Select an Account" description="Choose a customer or supplier from the dropdown above to inspect their ledger." />
        </motion.div>
      )}
    </motion.div>
  );
}

function LedgerTable({ entries }: { entries: LedgerEntry[], role: string }) {
  if (entries.length === 0) {
    return <EmptyState icon={Receipt} title="No Transactions" description="There are no ledger entries found for this account in the current role." />;
  }

  return (
    <div className="overflow-x-auto rounded-xl border border-slate-200/80 bg-white/50">
      <Table>
        <TableHeader className="bg-slate-100/80">
          <TableRow className="border-slate-200">
            <TableHead className="text-xs font-bold uppercase text-slate-700">Date</TableHead>
            <TableHead className="text-xs font-bold uppercase text-slate-700">Description</TableHead>
            <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Debit ($)</TableHead>
            <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Credit ($)</TableHead>
            <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Balance ($)</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {entries.map((entry, idx) => (
            <TableRow key={idx} className="border-slate-100 hover:bg-blue-50/30">
              <TableCell className="font-semibold text-slate-800">{entry.date.split('T')[0]}</TableCell>
              <TableCell className="capitalize font-bold text-slate-900">{entry.description.replace('_', ' ')}</TableCell>
              <TableCell className="text-right font-medium text-slate-700">{entry.debit > 0 ? `$${entry.debit.toFixed(2)}` : "-"}</TableCell>
              <TableCell className="text-right font-medium text-slate-700">{entry.credit > 0 ? `$${entry.credit.toFixed(2)}` : "-"}</TableCell>
              <TableCell className="text-right font-extrabold text-blue-600">${entry.balance.toFixed(2)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

