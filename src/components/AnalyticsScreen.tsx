import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Printer, TrendingUp, DollarSign, Activity, PieChart } from "lucide-react";
import { fmt } from "@/lib/format";
import { motion } from "framer-motion";

interface DashboardRollup {
  cash_in_hand: number;
  monthly_sales: number;
  net_profit: number;
}

interface IncomeStatement {
  revenue: number;
  cogs: number;
  gross_profit: number;
  operating_expenses: number;
  net_profit: number;
}

interface BalanceSheet {
  cash: number;
  inventory_value: number;
  fixed_assets: number;
  accounts_receivable: number;
  accounts_payable: number;
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

export function AnalyticsScreen() {
  const [dashboard, setDashboard] = useState<any>(null);
  const [income, setIncome] = useState<any>(null);
  const [balance, setBalance] = useState<any>(null);

  const loadData = async () => {
    try {
      setDashboard(await invoke<DashboardRollup>("get_financial_dashboard"));
      setIncome(await invoke<IncomeStatement>("get_income_statement"));
      setBalance(await invoke<BalanceSheet>("get_balance_sheet"));
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handlePrint = () => {
    window.print();
  };

  return (
    <motion.div 
      variants={containerVars}
      initial="hidden"
      animate="show"
      className="grid grid-cols-12 gap-6 h-full auto-rows-max print-container"
    >
      {/* HEADER BAR */}
      <motion.div variants={itemVars} className="col-span-12 glass-widget p-6 hover-lift flex flex-col md:flex-row md:items-center justify-between gap-4 no-print">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-2xl bg-blue-50 border border-blue-100 flex items-center justify-center shadow-xs">
            <PieChart className="text-blue-600 h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-extrabold text-slate-900 leading-tight">
              Financial Analytics & Statements
            </h1>
            <p className="text-slate-500 text-sm font-medium">Admin-only performance rollups, financial statements, and asset registers</p>
          </div>
        </div>
        <Button onClick={handlePrint} variant="outline" className="bg-white/80 border-slate-200 text-slate-800 font-bold hover:bg-slate-50 cursor-pointer">
          <Printer className="h-4 w-4 mr-2" /> Print / Export PDF
        </Button>
      </motion.div>

      {/* DASHBOARD HERO STATS CARDS */}
      <motion.div variants={itemVars} className="col-span-12 md:col-span-4 glass-widget p-6 hover-lift no-print">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-bold uppercase tracking-wider text-slate-500">Cash in Hand</span>
          <div className="w-8 h-8 rounded-xl bg-emerald-50 border border-emerald-100 flex items-center justify-center">
            <DollarSign className="h-4 w-4 text-emerald-600" />
          </div>
        </div>
        <h2 className="text-3xl font-extrabold text-slate-900">{fmt(dashboard?.cash_in_hand || 0)}</h2>
        <p className="text-xs font-semibold text-slate-500 mt-1">Physical vault & bank balance</p>
      </motion.div>

      <motion.div variants={itemVars} className="col-span-12 md:col-span-4 glass-widget p-6 hover-lift no-print">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-bold uppercase tracking-wider text-slate-500">Monthly Sales</span>
          <div className="w-8 h-8 rounded-xl bg-blue-50 border border-blue-100 flex items-center justify-center">
            <Activity className="h-4 w-4 text-blue-600" />
          </div>
        </div>
        <h2 className="text-3xl font-extrabold text-slate-900">{fmt(dashboard?.monthly_sales || 0)}</h2>
        <p className="text-xs font-semibold text-slate-500 mt-1">Current calendar month revenue</p>
      </motion.div>

      <motion.div variants={itemVars} className="col-span-12 md:col-span-4 glass-widget p-6 hover-lift no-print">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-bold uppercase tracking-wider text-slate-500">All-Time Net Profit</span>
          <div className="w-8 h-8 rounded-xl bg-indigo-50 border border-indigo-100 flex items-center justify-center">
            <TrendingUp className="h-4 w-4 text-indigo-600" />
          </div>
        </div>
        <h2 className={`text-3xl font-extrabold ${(dashboard?.net_profit || 0) >= 0 ? 'text-emerald-600' : 'text-rose-600'}`}>
          {fmt(dashboard?.net_profit || 0)}
        </h2>
        <p className="text-xs font-semibold text-slate-500 mt-1">Bottom line after COGS & Expenses</p>
      </motion.div>

      {/* TABS CONTAINER */}
      <motion.div variants={itemVars} className="col-span-12 glass-widget p-6 hover-lift">
        <Tabs defaultValue="pl" className="w-full">
          <TabsList className="bg-slate-100/80 border border-slate-200/80 p-1 rounded-xl mb-6 no-print">
            <TabsTrigger value="pl" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Income Statement (P&L)</TabsTrigger>
            <TabsTrigger value="bs" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Balance Sheet</TabsTrigger>
          </TabsList>

          {/* INCOME STATEMENT */}
          <TabsContent value="pl">
            <div className="max-w-3xl mx-auto p-6 rounded-2xl bg-white/80 border border-slate-200 shadow-sm print-container">
              <div className="text-center border-b border-slate-200 pb-4 mb-6">
                <h2 className="text-2xl font-extrabold text-slate-900">Income Statement (P&L)</h2>
                <p className="text-xs text-slate-500 font-semibold mt-1">Generated on {new Date().toLocaleDateString()}</p>
              </div>
              {income && (
                <div className="space-y-4 text-slate-800 font-semibold">
                  <div className="flex justify-between text-base">
                    <span className="text-slate-600">Sales Revenue</span>
                    <span className="font-extrabold text-slate-900">{fmt(income.revenue)}</span>
                  </div>
                  <div className="flex justify-between text-base text-rose-600">
                    <span>Cost of Goods Sold (COGS)</span>
                    <span>(-{fmt(income.cogs)})</span>
                  </div>
                  <div className="flex justify-between font-extrabold text-xl border-t border-b border-slate-200 py-3 bg-slate-50/70 px-3 rounded-lg text-slate-900">
                    <span>Gross Profit</span>
                    <span className="text-blue-600">{fmt(income.gross_profit)}</span>
                  </div>
                  <div className="flex justify-between text-base text-rose-600">
                    <span>Operating Expenses</span>
                    <span>(-{fmt(income.operating_expenses)})</span>
                  </div>
                  <div className="flex justify-between font-extrabold text-2xl border-b-4 border-double border-slate-900 py-3 mt-4 text-blue-600 px-3">
                    <span>Net Profit</span>
                    <span>{fmt(income.net_profit)}</span>
                  </div>
                </div>
              )}
            </div>
          </TabsContent>

          {/* BALANCE SHEET */}
          <TabsContent value="bs">
            <div className="max-w-4xl mx-auto p-6 rounded-2xl bg-white/80 border border-slate-200 shadow-sm print-container">
              <div className="text-center border-b border-slate-200 pb-4 mb-6">
                <h2 className="text-2xl font-extrabold text-slate-900">Balance Sheet</h2>
                <p className="text-xs text-slate-500 font-semibold mt-1">Generated on {new Date().toLocaleDateString()}</p>
              </div>
              {balance && (
                <div className="grid md:grid-cols-2 gap-8">
                  <div className="space-y-4 bg-slate-50/70 p-5 rounded-xl border border-slate-200">
                    <h3 className="font-extrabold text-lg text-slate-900 border-b border-slate-200 pb-2">Assets</h3>
                    <div className="space-y-3 text-sm font-semibold">
                      <div className="flex justify-between text-slate-700"><span>Cash / Bank</span><span className="font-bold text-slate-900">{fmt(balance.cash)}</span></div>
                      <div className="flex justify-between text-slate-700"><span>Accounts Receivable</span><span className="font-bold text-slate-900">{fmt(balance.accounts_receivable)}</span></div>
                      <div className="flex justify-between text-slate-700"><span>Inventory Value (Cost)</span><span className="font-bold text-slate-900">{fmt(balance.inventory_value)}</span></div>
                      <div className="flex justify-between text-slate-500"><span>Fixed Assets (Depreciated)</span><span className="font-bold text-slate-900">{fmt(balance.fixed_assets)}</span></div>
                      <div className="flex justify-between font-extrabold text-base pt-3 border-t border-slate-200 text-blue-600">
                        <span>Total Assets</span>
                        <span>{fmt(balance.cash + balance.accounts_receivable + balance.inventory_value + balance.fixed_assets)}</span>
                      </div>
                    </div>
                  </div>
                  <div className="space-y-4 bg-slate-50/70 p-5 rounded-xl border border-slate-200">
                    <h3 className="font-extrabold text-lg text-slate-900 border-b border-slate-200 pb-2">Liabilities & Equity</h3>
                    <div className="space-y-3 text-sm font-semibold">
                      <div className="flex justify-between text-slate-700"><span>Accounts Payable</span><span className="font-bold text-slate-900">{fmt(balance.accounts_payable)}</span></div>
                      <div className="flex justify-between text-slate-700 mt-6 pt-4 border-t border-slate-200"><span>Retained Earnings (Net Profit)</span><span className="font-bold text-blue-600">{fmt(income?.net_profit ?? 0)}</span></div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </TabsContent>


        </Tabs>
      </motion.div>
    </motion.div>
  );
}

