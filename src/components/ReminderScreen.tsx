import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Bell, Plus, Calendar, AlertCircle } from "lucide-react";
import { EmptyState } from "./ui/EmptyState";

interface Account {
  id: number;
  name: string;
  account_type: string;
}

interface Reminder {
  id?: number;
  account_id: number;
  note: string;
  due_date: string;
  status: string;
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

export function ReminderScreen() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [reminders, setReminders] = useState<Reminder[]>([]);
  
  const [accountId, setAccountId] = useState("");
  const [note, setNote] = useState("");
  const [dueDate, setDueDate] = useState<string>(new Date().toISOString().split("T")[0]);

  const loadData = async () => {
    try {
      const acc = await invoke<Account[]>("get_accounts");
      const rem = await invoke<Reminder[]>("get_reminders");
      setAccounts(acc);
      setReminders(rem);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleCreateReminder = async () => {
    if (!accountId || !note || !dueDate) return;
    try {
      await invoke("create_reminder", {
        payload: { account_id: parseInt(accountId), note, due_date: dueDate, status: "pending" },
      });
      setNote("");
      setAccountId("");
      loadData();
    } catch (err) {
      alert("Error: " + err);
    }
  };

  const getAccountName = (id: number) => {
    return accounts.find((a) => a.id === id)?.name || "Unknown Account";
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
            <Bell className="text-blue-600 h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-extrabold text-slate-900 leading-tight">
              Reminders & Follow-Up Log
            </h1>
            <p className="text-slate-500 text-sm font-medium">Track payment promises, pending calls, and task deadlines</p>
          </div>
        </div>
      </motion.div>

      {/* FORM CARD */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-4 glass-widget p-6 hover-lift flex flex-col justify-between">
        <div>
          <h2 className="text-lg font-bold text-slate-900 border-b border-slate-200 pb-3 mb-4">New Reminder</h2>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Account / Contact</label>
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
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Note / Payment Promise</label>
              <Input value={note} onChange={(e) => setNote(e.target.value)} placeholder="e.g. Promised to pay balance by Friday" className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Due Date</label>
              <Input type="date" value={dueDate} onChange={(e) => setDueDate(e.target.value)} className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
            </div>
          </div>
        </div>

        <div className="pt-6">
          <Button onClick={handleCreateReminder} className="w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold shadow-md shadow-blue-600/20 transition-all cursor-pointer">
            <Plus className="h-4 w-4 mr-2" /> Add Reminder
          </Button>
        </div>
      </motion.div>

      {/* REMINDERS LIST CARD */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-8 glass-widget p-6 hover-lift flex flex-col">
        <h2 className="text-lg font-bold text-slate-900 border-b border-slate-200 pb-3 mb-4">Upcoming & Pending Log</h2>
        {reminders.length === 0 ? (
          <EmptyState icon={Bell} title="No Pending Reminders" description="All follow-up tasks are clear." />
        ) : (
          <div className="space-y-3 overflow-y-auto max-h-[500px] pr-1">
            {reminders.map((rem, idx) => {
              const isOverdue = new Date(rem.due_date) < new Date();
              return (
                <div key={idx} className={`p-4 rounded-xl border flex items-center justify-between transition-all ${isOverdue ? 'bg-rose-50/70 border-rose-200 text-rose-900' : 'bg-white/80 border-slate-200 text-slate-900'}`}>
                  <div className="flex items-start gap-3">
                    <div className={`p-2 rounded-lg mt-0.5 ${isOverdue ? 'bg-rose-100 text-rose-600' : 'bg-blue-50 text-blue-600'}`}>
                      {isOverdue ? <AlertCircle className="h-4 w-4" /> : <Calendar className="h-4 w-4" />}
                    </div>
                    <div>
                      <p className="font-extrabold text-sm text-slate-900">{getAccountName(rem.account_id)}</p>
                      <p className="text-xs font-medium text-slate-600 mt-0.5">{rem.note}</p>
                    </div>
                  </div>
                  <div className="text-right">
                    <span className={`text-xs font-bold px-2.5 py-1 rounded-full ${isOverdue ? 'bg-rose-100 text-rose-700' : 'bg-slate-100 text-slate-700'}`}>
                      {rem.due_date.split('T')[0]}
                    </span>
                    {isOverdue && (
                      <span className="block text-[10px] uppercase font-extrabold text-rose-600 mt-1">Overdue</span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </motion.div>
    </motion.div>
  );
}

