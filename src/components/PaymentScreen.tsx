import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "./ui/EmptyState";
import { Receipt } from "lucide-react";

interface Account {
  id: number;
  name: string;
  account_type: string;
}

interface Payment {
  id?: number;
  account_id: number;
  payment_type: string;
  amount: number;
  tid: string;
  document_date: string;
  bank_cash_name: string;
}

export function PaymentScreen() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [payments, setPayments] = useState<Payment[]>([]);

  const [accountId, setAccountId] = useState("");
  const [paymentType, setPaymentType] = useState("receipt");
  const [amount, setAmount] = useState<number | "">("");
  const [tid, setTid] = useState("");
  const [documentDate, setDocumentDate] = useState<string>(new Date().toISOString().split("T")[0]);
  const [bankCashName, setBankCashName] = useState("");

  const loadData = async () => {
    try {
      const acc = await invoke<Account[]>("get_accounts");
      const pay = await invoke<Payment[]>("get_payments");
      setAccounts(acc && acc.length > 0 ? acc : [{id: 1, name: "Test Account", account_type: "customer"}]);
      setPayments(pay || []);
    } catch (err) {
      console.error(err);
      // Fallback for UI testing
      setAccounts([{id: 1, name: "Test Account", account_type: "customer"}]);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleLogPayment = async () => {
    if (!accountId || !amount || !tid || !documentDate || !bankCashName) {
      alert("Please fill all fields, including the unique Transaction ID (TID) and Bank/Cash Name.");
      return;
    }
    
    try {
      const payload = {
        account_id: parseInt(accountId),
        payment_type: paymentType,
        amount: Number(amount),
        tid,
        document_date: documentDate,
        bank_cash_name: bankCashName,
      };

      await invoke("create_payment", { payload });
      
      // Reset form
      setAmount("");
      setTid("");
      setBankCashName("");
      loadData();
    } catch (err) {
      // This will catch the Duplicate TID Protection error from Rust
      alert(err);
    }
  };

  const getAccountName = (id: number) => {
    return accounts.find((a) => a.id === id)?.name || "Unknown";
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2">
        <h2 className="text-3xl font-bold tracking-tight text-primary">Payments & Receipts</h2>
        <p className="text-muted-foreground">Log incoming receipts and outgoing payments with strict duplicate TID protection.</p>
      </div>

      <div className="grid md:grid-cols-3 gap-6">
        <Card className="md:col-span-1 h-fit hover-lift">
          <CardHeader>
            <CardTitle>Log Transaction</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Transaction Type</label>
              <Select value={paymentType} onValueChange={setPaymentType}>
                <SelectTrigger>
                  <SelectValue placeholder="Select Type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="receipt">Money In (Receipt)</SelectItem>
                  <SelectItem value="payment">Money Out (Payment)</SelectItem>
                </SelectContent>
              </Select>
            </div>
            
            <div className="space-y-2">
              <label className="text-sm font-medium">Contact / Account</label>
              <Select value={accountId} onValueChange={setAccountId}>
                <SelectTrigger>
                  <SelectValue placeholder="Select Account" />
                </SelectTrigger>
                <SelectContent>
                  {accounts.map(acc => (
                    <SelectItem key={acc.id} value={acc.id.toString()}>{acc.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Amount</label>
                <Input type="number" value={amount} onChange={(e) => setAmount(parseFloat(e.target.value) || "")} />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Date</label>
                <Input type="date" value={documentDate} onChange={(e) => setDocumentDate(e.target.value)} />
              </div>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-destructive">Transaction ID (TID)</label>
              <Input 
                value={tid} 
                onChange={(e) => setTid(e.target.value)} 
                placeholder="Unique Ref No. / Cheque No." 
                className="border-destructive focus-visible:ring-destructive"
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium">Bank / Cash Name</label>
              <Input value={bankCashName} onChange={(e) => setBankCashName(e.target.value)} placeholder="e.g. HBL Main Branch" />
            </div>

            <Button className="w-full" onClick={handleLogPayment}>Save Payment</Button>
          </CardContent>
        </Card>

        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Recent Transactions</CardTitle>
          </CardHeader>
          <CardContent>
            {payments.length === 0 ? (
              <EmptyState icon={Receipt} title="No Payments Logged" description="You have not recorded any payments or receipts yet." />
            ) : (
              <div className="rounded-md border">
                <Table>
                  <TableHeader className="bg-muted/50">
                    <TableRow>
                      <TableHead>Date</TableHead>
                      <TableHead>TID Ref</TableHead>
                      <TableHead>Account</TableHead>
                      <TableHead>Type</TableHead>
                      <TableHead>Bank/Cash</TableHead>
                      <TableHead className="text-right">Amount</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {payments.map((pay, idx) => (
                      <TableRow key={idx}>
                        <TableCell>{pay.document_date.split('T')[0]}</TableCell>
                        <TableCell className="font-mono text-xs">{pay.tid}</TableCell>
                        <TableCell className="font-medium">{getAccountName(pay.account_id)}</TableCell>
                        <TableCell className="capitalize">{pay.payment_type}</TableCell>
                        <TableCell>{pay.bank_cash_name}</TableCell>
                        <TableCell className={`text-right font-medium ${pay.payment_type === 'receipt' ? 'text-emerald-600' : 'text-destructive'}`}>
                          {pay.amount.toFixed(2)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
