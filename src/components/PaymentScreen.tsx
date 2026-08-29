import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { EmptyState } from "./ui/EmptyState";
import { Receipt, ShoppingBag, Plus } from "lucide-react";
import { fmt } from "@/lib/format";

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
  settlement_account_code: string;
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

export function PaymentScreen() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [payments, setPayments] = useState<Payment[]>([]);

  const [accountId, setAccountId] = useState("");
  const [paymentType, setPaymentType] = useState("receipt");
  const [amount, setAmount] = useState<number | "">("");
  const [tid, setTid] = useState("");
  const [documentDate, setDocumentDate] = useState<string>(new Date().toISOString().split("T")[0]);

  const loadData = async () => {
    try {
      const acc = await invoke<Account[]>("get_accounts");
      const pay = await invoke<Payment[]>("get_payments");
      setAccounts(acc && acc.length > 0 ? acc : [{id: 1, name: "Test Account", account_type: "customer"}]);
      setPayments(pay || []);
    } catch (err) {
      console.error(err);
      setAccounts([{id: 1, name: "Test Account", account_type: "customer"}]);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleLogPayment = async () => {
    if (!accountId || !amount || !tid || !documentDate) {
      alert("Please fill all fields, including the unique Transaction ID (TID).");
      return;
    }
    
    try {
      const payload = {
        account_id: parseInt(accountId),
        payment_type: paymentType,
        amount: Number(amount),
        tid,
        document_date: documentDate,
        settlement_account_code: "1000", // Defaulting to main Cash/Bank account
      };

      await invoke("create_payment", { payload });
      
      setAmount("");
      setTid("");
      loadData();
    } catch (err) {
      alert(err);
    }
  };

  const getAccountName = (id: number) => {
    return accounts.find((a) => a.id === id)?.name || "Unknown";
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
            <ShoppingBag className="text-blue-600 h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-extrabold text-slate-900 leading-tight">
              Payments & Receipts
            </h1>
            <p className="text-slate-500 text-sm font-medium">Log incoming receipts and outgoing payments with strict duplicate TID protection</p>
          </div>
        </div>
      </motion.div>

      {/* FORM WIDGET */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-4 glass-widget p-6 hover-lift flex flex-col justify-between">
        <div>
          <h2 className="text-lg font-bold text-slate-900 border-b border-slate-200 pb-3 mb-4">Log Transaction</h2>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Transaction Type</label>
              <Select value={paymentType} onValueChange={setPaymentType}>
                <SelectTrigger className="bg-white/80 border-slate-200 focus:ring-blue-500 text-slate-900 font-semibold">
                  <SelectValue placeholder="Select Type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="receipt">Money In (Receipt)</SelectItem>
                  <SelectItem value="payment">Money Out (Payment)</SelectItem>
                </SelectContent>
              </Select>
            </div>
            
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Contact / Account</label>
              <Select value={accountId} onValueChange={setAccountId}>
                <SelectTrigger className="bg-white/80 border-slate-200 focus:ring-blue-500 text-slate-900 font-semibold">
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
              <div className="space-y-1.5">
                <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Amount ($)</label>
                <Input type="number" value={amount} onChange={(e) => setAmount(parseFloat(e.target.value) || "")} className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
              </div>
              <div className="space-y-1.5">
                <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Date</label>
                <Input type="date" value={documentDate} onChange={(e) => setDocumentDate(e.target.value)} className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
              </div>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-rose-600">Transaction ID (TID / Ref)</label>
              <Input 
                value={tid} 
                onChange={(e) => setTid(e.target.value)} 
                placeholder="Unique Ref No. / Cheque No." 
                className="bg-white/80 border-rose-200 focus:ring-rose-500 text-slate-900 font-semibold"
              />
            </div>
          </div>
        </div>

        <div className="pt-6">
          <Button onClick={handleLogPayment} className="w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold shadow-md shadow-blue-600/20 transition-all cursor-pointer">
            <Plus className="h-4 w-4 mr-2" /> Save Payment
          </Button>
        </div>
      </motion.div>

      {/* TABLE WIDGET */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-8 glass-widget p-6 hover-lift flex flex-col">
        <h2 className="text-lg font-bold text-slate-900 border-b border-slate-200 pb-3 mb-4">Recent Transactions</h2>
        {payments.length === 0 ? (
          <EmptyState icon={Receipt} title="No Payments Logged" description="You have not recorded any payments or receipts yet." />
        ) : (
          <div className="overflow-x-auto rounded-xl border border-slate-200/80 bg-white/50">
            <Table>
              <TableHeader className="bg-slate-100/80">
                <TableRow className="border-slate-200">
                  <TableHead className="text-xs font-bold uppercase text-slate-700">Date</TableHead>
                  <TableHead className="text-xs font-bold uppercase text-slate-700">TID Ref</TableHead>
                  <TableHead className="text-xs font-bold uppercase text-slate-700">Account</TableHead>
                  <TableHead className="text-xs font-bold uppercase text-slate-700">Type</TableHead>
                  <TableHead className="text-xs font-bold uppercase text-slate-700">Bank/Cash</TableHead>
                  <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Amount</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {payments.map((pay, idx) => (
                  <TableRow key={idx} className="border-slate-100 hover:bg-blue-50/30">
                    <TableCell className="font-semibold text-slate-800">{pay.document_date.split('T')[0]}</TableCell>
                    <TableCell className="font-mono text-xs font-bold text-slate-600">{pay.tid}</TableCell>
                    <TableCell className="font-bold text-slate-900">{getAccountName(pay.account_id)}</TableCell>
                    <TableCell>
                      <span className={`text-xs font-extrabold px-2.5 py-1 rounded-full uppercase ${pay.payment_type === 'receipt' ? 'bg-emerald-50 text-emerald-700 border border-emerald-200' : 'bg-rose-50 text-rose-700 border border-rose-200'}`}>
                        {pay.payment_type}
                      </span>
                    </TableCell>
                    <TableCell className="text-slate-600 font-medium">{pay.settlement_account_code}</TableCell>
                    <TableCell className={`text-right font-extrabold text-base ${pay.payment_type === 'receipt' ? 'text-emerald-600' : 'text-rose-600'}`}>
                      {pay.payment_type === 'receipt' ? '+' : '-'}{fmt(pay.amount)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </motion.div>
    </motion.div>
  );
}

