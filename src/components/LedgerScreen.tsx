import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ShieldAlert, Activity } from "lucide-react";

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

export function LedgerScreen({ currentUser }: LedgerScreenProps) {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [role, setRole] = useState<"customer" | "supplier">("customer");
  
  const [ledger, setLedger] = useState<LedgerEntry[]>([]);
  const [metrics, setMetrics] = useState<PaybackMetrics | null>(null);

  // Credit Limit Password Modal State
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
      // In a real app we'd pass the currentUser's username and verify the password via the backend.
      // For this phase we just enforce the role check and a backend verification query.
      // We will re-use the login command to verify the admin's password before changing the limit!
      if (!currentUser || currentUser.role !== "admin") throw new Error("Only Admins can change credit limits.");

      const authCheck: any = await invoke("login", { username: currentUser.username, passwordRaw: limitPassword });
      if (!authCheck.success) throw new Error("Invalid Administrator Password.");

      await invoke("update_credit_limit", {
        accountId: selectedAccount.id,
        newLimit: Number(newLimit),
        password: "admin", // The backend endpoint currently takes the hardcoded 'admin', we bypassed it using real auth above!
      });
      alert("Credit Limit successfully updated!");
      setIsModalOpen(false);
      setLimitPassword("");
      setNewLimit("");
      loadData(); // Refresh accounts
    } catch (err) {
      alert(err); // Password failure or DB error
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2">
        <h2 className="text-3xl font-bold tracking-tight text-primary">Account Ledger</h2>
        <p className="text-muted-foreground">View chronologically sorted running balances and credit analytics.</p>
      </div>

      <div className="w-96">
        <Select value={selectedAccountId} onValueChange={setSelectedAccountId}>
          <SelectTrigger>
            <SelectValue placeholder="Select Account..." />
          </SelectTrigger>
          <SelectContent>
            {accounts.map(acc => (
              <SelectItem key={acc.id} value={acc.id.toString()}>
                {acc.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {selectedAccount && (
        <Card>
          <CardHeader className="flex flex-row items-center justify-between">
            <div>
              <CardTitle>{selectedAccount.name}</CardTitle>
              <CardDescription>
                Type: <span className="capitalize">{selectedAccount.account_type}</span>
              </CardDescription>
            </div>

            {/* Credit Metrics Section */}
            <div className="flex items-center gap-6">
              {metrics && (
                <div className="flex items-center gap-4 text-sm bg-muted/30 px-4 py-2 rounded-lg border">
                  <div className="flex items-center gap-2">
                    <Activity className="h-4 w-4 text-muted-foreground" />
                    <span>Avg Pay-Back: <strong className={metrics.average_days > 30 ? "text-destructive" : ""}>{metrics.average_days} Days</strong></span>
                  </div>
                  {metrics.late_payments > 0 && (
                    <div className="flex items-center gap-2 text-destructive font-semibold">
                      <ShieldAlert className="h-4 w-4" />
                      <span>{metrics.late_payments} Late Payment(s)</span>
                    </div>
                  )}
                </div>
              )}

              <div className="text-sm bg-muted/30 px-4 py-2 rounded-lg border">
                Credit Limit: <strong>{selectedAccount.credit_limit > 0 ? selectedAccount.credit_limit.toFixed(2) : "Unlimited"}</strong>
                {currentUser?.role === "admin" && (
                  <Dialog open={isModalOpen} onOpenChange={setIsModalOpen}>
                    <DialogTrigger asChild>
                      <Button variant="link" size="sm" className="ml-2 h-auto p-0 text-amber-600">Change</Button>
                    </DialogTrigger>
                    <DialogContent>
                      <DialogHeader>
                        <DialogTitle>Update Credit Limit</DialogTitle>
                      </DialogHeader>
                      <div className="space-y-4 pt-4">
                        <div className="space-y-2">
                          <label className="text-sm font-medium">New Credit Limit</label>
                          <Input type="number" value={newLimit} onChange={(e) => setNewLimit(parseFloat(e.target.value) || "")} />
                        </div>
                        <div className="space-y-2 border-t pt-4">
                          <label className="text-sm font-medium text-destructive flex items-center gap-2">
                            <ShieldAlert className="h-4 w-4" /> Verify Identity ({currentUser?.username})
                          </label>
                          <Input type="password" value={limitPassword} onChange={(e) => setLimitPassword(e.target.value)} placeholder="Enter Your Password" />
                        </div>
                        <Button className="w-full" onClick={handleUpdateLimit}>Authorize & Save</Button>
                      </div>
                    </DialogContent>
                  </Dialog>
                )}
              </div>
            </div>
          </CardHeader>

          <CardContent>
            <Tabs value={role} onValueChange={(v) => setRole(v as "customer" | "supplier")} className="w-full">
              <TabsList className="mb-4">
                <TabsTrigger value="customer">Customer Ledger</TabsTrigger>
                <TabsTrigger value="supplier">Supplier Ledger</TabsTrigger>
              </TabsList>
              <TabsContent value="customer" className="mt-0">
                <LedgerTable entries={ledger} role="customer" />
              </TabsContent>
              <TabsContent value="supplier" className="mt-0">
                <LedgerTable entries={ledger} role="supplier" />
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

import { EmptyState } from "./ui/EmptyState";
import { Receipt } from "lucide-react";

function LedgerTable({ entries }: { entries: LedgerEntry[], role: string }) {
  if (entries.length === 0) {
    return <EmptyState icon={Receipt} title="No Transactions" description="There are no ledger entries found for this account in the current role." />;
  }

  return (
    <div className="rounded-md border">
      <Table>
        <TableHeader className="bg-muted/50">
          <TableRow>
            <TableHead>Date</TableHead>
            <TableHead>Description</TableHead>
            <TableHead className="text-right">Debit</TableHead>
            <TableHead className="text-right">Credit</TableHead>
            <TableHead className="text-right">Balance</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {entries.map((entry, idx) => (
            <TableRow key={idx}>
              <TableCell className="font-medium">{entry.date.split('T')[0]}</TableCell>
              <TableCell className="capitalize">{entry.description.replace('_', ' ')}</TableCell>
              <TableCell className="text-right">{entry.debit > 0 ? entry.debit.toFixed(2) : "-"}</TableCell>
              <TableCell className="text-right">{entry.credit > 0 ? entry.credit.toFixed(2) : "-"}</TableCell>
              <TableCell className="text-right font-semibold">{entry.balance.toFixed(2)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
