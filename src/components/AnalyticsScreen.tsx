import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Printer, TrendingUp, DollarSign, Activity, PieChart, Plus } from "lucide-react";
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

interface FixedAsset {
  id?: number;
  name: string;
  value: number;
  depreciation_rate: number;
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
  const [assets, setAssets] = useState<any[]>([]);
  const [assetName, setAssetName] = useState("");
  const [assetValue, setAssetValue] = useState<number | "">("");
  const [depRate, setDepRate] = useState<number | "">("");

  const loadData = async () => {
    try {
      setDashboard(await invoke<DashboardRollup>("get_financial_dashboard"));
      setIncome(await invoke<IncomeStatement>("get_income_statement"));
      setBalance(await invoke<BalanceSheet>("get_balance_sheet"));
      setAssets(await invoke<FixedAsset[]>("get_assets"));
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

  const handleRegisterAsset = async () => {
    if (!assetName || assetValue === "" || depRate === "") return;
    try {
      await invoke("create_asset", {
        payload: {
          name: assetName,
          value: Number(assetValue),
          depreciation_rate: Number(depRate)
        }
      });
      setAssetName(""); setAssetValue(""); setDepRate("");
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
        <h2 className="text-3xl font-extrabold text-slate-900">${(dashboard?.cash_in_hand || 0).toFixed(2)}</h2>
        <p className="text-xs font-semibold text-slate-500 mt-1">Physical vault & bank balance</p>
      </motion.div>

      <motion.div variants={itemVars} className="col-span-12 md:col-span-4 glass-widget p-6 hover-lift no-print">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-bold uppercase tracking-wider text-slate-500">Monthly Sales</span>
          <div className="w-8 h-8 rounded-xl bg-blue-50 border border-blue-100 flex items-center justify-center">
            <Activity className="h-4 w-4 text-blue-600" />
          </div>
        </div>
        <h2 className="text-3xl font-extrabold text-slate-900">${(dashboard?.monthly_sales || 0).toFixed(2)}</h2>
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
          ${(dashboard?.net_profit || 0).toFixed(2)}
        </h2>
        <p className="text-xs font-semibold text-slate-500 mt-1">Bottom line after COGS & Expenses</p>
      </motion.div>

      {/* TABS CONTAINER */}
      <motion.div variants={itemVars} className="col-span-12 glass-widget p-6 hover-lift">
        <Tabs defaultValue="pl" className="w-full">
          <TabsList className="bg-slate-100/80 border border-slate-200/80 p-1 rounded-xl mb-6 no-print">
            <TabsTrigger value="pl" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Income Statement (P&L)</TabsTrigger>
            <TabsTrigger value="bs" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Balance Sheet</TabsTrigger>
            <TabsTrigger value="assets" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Fixed Assets</TabsTrigger>
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
                    <span className="font-extrabold text-slate-900">${income.revenue.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between text-base text-rose-600">
                    <span>Cost of Goods Sold (COGS)</span>
                    <span>(-${income.cogs.toFixed(2)})</span>
                  </div>
                  <div className="flex justify-between font-extrabold text-xl border-t border-b border-slate-200 py-3 bg-slate-50/70 px-3 rounded-lg text-slate-900">
                    <span>Gross Profit</span>
                    <span className="text-blue-600">${income.gross_profit.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between text-base text-rose-600">
                    <span>Operating Expenses</span>
                    <span>(-${income.operating_expenses.toFixed(2)})</span>
                  </div>
                  <div className="flex justify-between font-extrabold text-2xl border-b-4 border-double border-slate-900 py-3 mt-4 text-blue-600 px-3">
                    <span>Net Profit</span>
                    <span>${income.net_profit.toFixed(2)}</span>
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
                      <div className="flex justify-between text-slate-700"><span>Cash / Bank</span><span className="font-bold text-slate-900">${balance.cash.toFixed(2)}</span></div>
                      <div className="flex justify-between text-slate-700"><span>Accounts Receivable</span><span className="font-bold text-slate-900">${balance.accounts_receivable.toFixed(2)}</span></div>
                      <div className="flex justify-between text-slate-700"><span>Inventory Value (Cost)</span><span className="font-bold text-slate-900">${balance.inventory_value.toFixed(2)}</span></div>
                      <div className="flex justify-between text-slate-500"><span>Fixed Assets (Depreciated)</span><span className="font-bold text-slate-900">${balance.fixed_assets.toFixed(2)}</span></div>
                      <div className="flex justify-between font-extrabold text-base pt-3 border-t border-slate-200 text-blue-600">
                        <span>Total Assets</span>
                        <span>${(balance.cash + balance.accounts_receivable + balance.inventory_value + balance.fixed_assets).toFixed(2)}</span>
                      </div>
                    </div>
                  </div>
                  <div className="space-y-4 bg-slate-50/70 p-5 rounded-xl border border-slate-200">
                    <h3 className="font-extrabold text-lg text-slate-900 border-b border-slate-200 pb-2">Liabilities & Equity</h3>
                    <div className="space-y-3 text-sm font-semibold">
                      <div className="flex justify-between text-slate-700"><span>Accounts Payable</span><span className="font-bold text-slate-900">${balance.accounts_payable.toFixed(2)}</span></div>
                      <div className="flex justify-between text-slate-700 mt-6 pt-4 border-t border-slate-200"><span>Retained Earnings (Net Profit)</span><span className="font-bold text-blue-600">${income?.net_profit.toFixed(2)}</span></div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </TabsContent>

          {/* FIXED ASSETS */}
          <TabsContent value="assets" className="no-print">
            <div className="grid grid-cols-12 gap-6">
              <div className="col-span-12 lg:col-span-4 bg-white/70 p-5 rounded-xl border border-slate-200 space-y-4">
                <h3 className="text-base font-bold text-slate-900 border-b border-slate-200 pb-2">Register Asset</h3>
                <div className="space-y-3">
                  <div className="space-y-1">
                    <label className="text-xs font-bold uppercase text-slate-600">Asset Name</label>
                    <Input value={assetName} onChange={(e) => setAssetName(e.target.value)} placeholder="e.g. Delivery Van" className="bg-white border-slate-200 font-semibold" />
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs font-bold uppercase text-slate-600">Value / Cost ($)</label>
                    <Input type="number" value={assetValue} onChange={(e) => setAssetValue(parseFloat(e.target.value) || "")} className="bg-white border-slate-200 font-semibold" />
                  </div>
                  <div className="space-y-1">
                    <label className="text-xs font-bold uppercase text-slate-600">Depreciation Rate (%)</label>
                    <Input type="number" value={depRate} onChange={(e) => setDepRate(parseFloat(e.target.value) || "")} placeholder="e.g. 10" className="bg-white border-slate-200 font-semibold" />
                  </div>
                  <Button className="w-full bg-blue-600 hover:bg-blue-700 text-white font-bold cursor-pointer mt-2" onClick={handleRegisterAsset}>
                    <Plus className="h-4 w-4 mr-2" /> Save Asset
                  </Button>
                </div>
              </div>

              <div className="col-span-12 lg:col-span-8 bg-white/70 p-5 rounded-xl border border-slate-200">
                <h3 className="text-base font-bold text-slate-900 border-b border-slate-200 pb-2 mb-4">Asset Register</h3>
                <div className="overflow-x-auto rounded-xl border border-slate-200 bg-white/80">
                  <Table>
                    <TableHeader className="bg-slate-100">
                      <TableRow>
                        <TableHead className="text-xs font-bold uppercase text-slate-700">Asset Name</TableHead>
                        <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Cost Value</TableHead>
                        <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Dep. Rate</TableHead>
                        <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Book Value</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {assets.map((a, idx) => {
                        const bookValue = a.value * (1 - (a.depreciation_rate / 100));
                        return (
                          <TableRow key={idx} className="hover:bg-blue-50/20 border-slate-100">
                            <TableCell className="font-bold text-slate-900">{a.name}</TableCell>
                            <TableCell className="text-right font-medium text-slate-700">${a.value.toFixed(2)}</TableCell>
                            <TableCell className="text-right font-semibold text-slate-600">{a.depreciation_rate}%</TableCell>
                            <TableCell className="text-right font-extrabold text-blue-600">${bookValue.toFixed(2)}</TableCell>
                          </TableRow>
                        );
                      })}
                      {assets.length === 0 && (
                        <TableRow>
                          <TableCell colSpan={4} className="text-center py-6 text-slate-500 font-medium">No fixed assets registered yet.</TableCell>
                        </TableRow>
                      )}
                    </TableBody>
                  </Table>
                </div>
              </div>
            </div>
          </TabsContent>
        </Tabs>
      </motion.div>
    </motion.div>
  );
}

