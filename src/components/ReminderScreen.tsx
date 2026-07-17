import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

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
    <div className="space-y-6">
      <div className="flex flex-col gap-2">
        <h2 className="text-3xl font-bold tracking-tight text-primary">Reminders & Follow-Ups</h2>
        <p className="text-muted-foreground">Keep track of overdue payments and follow-up tasks.</p>
      </div>

      <div className="grid md:grid-cols-3 gap-6">
        <Card className="md:col-span-1 h-fit">
          <CardHeader>
            <CardTitle>New Reminder</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Account</label>
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
            <div className="space-y-2">
              <label className="text-sm font-medium">Note / Promise</label>
              <Input value={note} onChange={(e) => setNote(e.target.value)} placeholder="e.g. Promised to pay by Friday" />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Due Date</label>
              <Input type="date" value={dueDate} onChange={(e) => setDueDate(e.target.value)} />
            </div>
            <Button className="w-full" onClick={handleCreateReminder}>Add Reminder</Button>
          </CardContent>
        </Card>

        <div className="md:col-span-2 space-y-4">
          <h3 className="text-xl font-semibold">Upcoming & Pending</h3>
          {reminders.length === 0 ? (
            <div className="p-8 border border-dashed rounded-lg text-center text-muted-foreground">
              No pending reminders.
            </div>
          ) : (
            <div className="grid gap-4">
              {reminders.map((rem, idx) => {
                const isOverdue = new Date(rem.due_date) < new Date();
                return (
                  <Card key={idx} className={isOverdue ? "border-destructive/50 bg-destructive/5" : ""}>
                    <CardContent className="p-4 flex items-center justify-between">
                      <div>
                        <p className="font-semibold">{getAccountName(rem.account_id)}</p>
                        <p className="text-sm text-muted-foreground">{rem.note}</p>
                      </div>
                      <div className="text-right">
                        <p className={`font-medium ${isOverdue ? "text-destructive" : ""}`}>
                          {rem.due_date.split('T')[0]}
                        </p>
                        {isOverdue && <span className="text-[10px] uppercase tracking-wider font-bold text-destructive">Overdue</span>}
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
