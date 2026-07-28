import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ShieldAlert, Activity, Receipt, Users } from "lucide-react";
import { EmptyState } from "./ui/EmptyState";
import { fmt } from "@/lib/format";

export interface Account {
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

interface PartyDetailProps {
  account: Account;
  currentUser?: { id: number; username: string; role: string };
  onLimitUpdated: () => void;
}

export function PartyDetail({ account, currentUser, onLimitUpdated }: PartyDetailProps) {
  const [role, setRole] = useState<"customer" | "supplier">("customer");
  const [ledger, setLedger] = useState<LedgerEntry[]>([]);
  const [metrics, setMetrics] = useState<PaybackMetrics | null>(null);
  const [limitPassword, setLimitPassword] = useState("");
  const [newLimit, setNewLimit] = useState<number | "">("");
  const [isModalOpen, setIsModalOpen] = useState(false);

  useEffect(() => {
    invoke<LedgerEntry[]>("get_account_ledger", { accountId: account.id, role })
      .then(setLedger)
      .catch(console.error);

    if (role === "customer") {
      invoke<PaybackMetrics>("analyze_payback_behavior", { accountId: account.id })
        .then(setMetrics)
        .catch(console.error);
    } else {
      setMetrics(null);
    }
  }, [account.id, role]);

  const handleUpdateLimit = async () => {
    if (newLimit === "") return;
    try {
      if (!currentUser || currentUser.role !== "admin") throw new Error("Only Admins can change credit limits.");
      const authCheck: any = await invoke("login", { username: currentUser.username, passwordRaw: limitPassword });
      if (!authCheck.success) throw new Error("Invalid Administrator Password.");
      await invoke("update_credit_limit", {
        accountId: account.id,
        newLimit: Number(newLimit),
        currentRole: currentUser.role,
      });
      alert("Credit Limit successfully updated!");
      setIsModalOpen(false);
      setLimitPassword("");
      setNewLimit("");
      onLimitUpdated();
    } catch (err) {
      alert(err);
    }
  };

  return (
    <div className="space-y-6">
      {/* Account header */}
      <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4 border-b border-slate-200 pb-4">
        <div>
          <h2 className="text-xl font-bold text-slate-900 flex items-center gap-2">
            <Users className="h-5 w-5 text-blue-600" />
            {account.name}
          </h2>
          <p className="text-xs font-semibold text-slate-500 uppercase tracking-wider mt-1">
            Account Type: <span className="text-blue-600">{account.account_type}</span>
          </p>
        </div>

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
            <strong className="text-slate-900 font-extrabold text-sm">
              {account.credit_limit > 0 ? fmt(account.credit_limit) : "Unlimited"}
            </strong>
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

      {/* Ledger tabs */}
      <Tabs value={role} onValueChange={(v) => setRole(v as "customer" | "supplier")} className="w-full">
        <TabsList className="bg-slate-100/80 border border-slate-200/80 p-1 rounded-xl mb-4">
          <TabsTrigger value="customer" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Customer Ledger</TabsTrigger>
          <TabsTrigger value="supplier" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Supplier Ledger</TabsTrigger>
        </TabsList>
        <TabsContent value="customer" className="mt-0">
          <LedgerTable entries={ledger} />
        </TabsContent>
        <TabsContent value="supplier" className="mt-0">
          <LedgerTable entries={ledger} />
        </TabsContent>
      </Tabs>
    </div>
  );
}

function LedgerTable({ entries }: { entries: LedgerEntry[] }) {
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
              <TableCell className="text-right font-medium text-slate-700">{entry.debit > 0 ? fmt(entry.debit) : "-"}</TableCell>
              <TableCell className="text-right font-medium text-slate-700">{entry.credit > 0 ? fmt(entry.credit) : "-"}</TableCell>
              <TableCell className="text-right font-extrabold text-blue-600">{fmt(entry.balance)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
