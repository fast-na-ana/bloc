import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { DollarSign, Plus } from "lucide-react";

interface Expense {
  id?: number;
  description: string;
  amount: number;
  date: string;
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

export function ExpenseScreen() {
  const [expenses, setExpenses] = useState<Expense[]>([]);
  const [description, setDescription] = useState("");
  const [amount, setAmount] = useState<number | "">("");
  const [date, setDate] = useState<string>(new Date().toISOString().split("T")[0]);

  const loadExpenses = async () => {
    try {
      const data = await invoke<Expense[]>("get_expenses");
      setExpenses(data);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadExpenses();
  }, []);

  const handleLogExpense = async () => {
    if (!description || !amount || !date) return;
    try {
      await invoke("create_expense", {
        payload: { description, amount: Number(amount), date },
      });
      setDescription("");
      setAmount("");
      loadExpenses();
    } catch (err) {
      alert("Error: " + err);
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
            <DollarSign className="text-blue-600 h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-extrabold text-slate-900 leading-tight">
              Expense Tracking
            </h1>
            <p className="text-slate-500 text-sm font-medium">Log day-to-day business operating expenses independently from purchases</p>
          </div>
        </div>
      </motion.div>

      {/* FORM CARD */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-4 glass-widget p-6 hover-lift flex flex-col justify-between">
        <div>
          <h2 className="text-lg font-bold text-slate-900 border-b border-slate-200 pb-3 mb-4">Log Expense</h2>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Description</label>
              <Input value={description} onChange={(e) => setDescription(e.target.value)} placeholder="e.g. Office Supplies / Utilities" className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Amount ($)</label>
              <Input type="number" value={amount} onChange={(e) => setAmount(parseFloat(e.target.value) || "")} className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Date</label>
              <Input type="date" value={date} onChange={(e) => setDate(e.target.value)} className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
            </div>
          </div>
        </div>

        <div className="pt-6">
          <Button onClick={handleLogExpense} className="w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold shadow-md shadow-blue-600/20 transition-all cursor-pointer">
            <Plus className="h-4 w-4 mr-2" /> Save Expense
          </Button>
        </div>
      </motion.div>

      {/* EXPENSE TABLE CARD */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-8 glass-widget p-6 hover-lift flex flex-col">
        <h2 className="text-lg font-bold text-slate-900 border-b border-slate-200 pb-3 mb-4">Recent Expenses</h2>
        <div className="overflow-x-auto rounded-xl border border-slate-200/80 bg-white/50">
          <Table>
            <TableHeader className="bg-slate-100/80">
              <TableRow className="border-slate-200">
                <TableHead className="text-xs font-bold uppercase text-slate-700">Date</TableHead>
                <TableHead className="text-xs font-bold uppercase text-slate-700">Description</TableHead>
                <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Amount ($)</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {expenses.map((exp, idx) => (
                <TableRow key={idx} className="border-slate-100 hover:bg-blue-50/30">
                  <TableCell className="font-semibold text-slate-800">{exp.date.split('T')[0]}</TableCell>
                  <TableCell className="font-bold text-slate-900">{exp.description}</TableCell>
                  <TableCell className="text-right font-extrabold text-rose-600">-${exp.amount.toFixed(2)}</TableCell>
                </TableRow>
              ))}
              {expenses.length === 0 && (
                <TableRow>
                  <TableCell colSpan={3} className="text-center py-8 text-slate-500 font-medium">No expenses logged yet.</TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
      </motion.div>
    </motion.div>
  );
}

