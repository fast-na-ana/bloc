import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Printer, TrendingUp, DollarSign, Activity } from "lucide-react";
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
    transition: { staggerChildren: 0.1 }
  }
} as any;

const itemVars = {
  hidden: { opacity: 0, y: 10 },
  show: { opacity: 1, y: 0, transition: { duration: 0.3, ease: "easeOut" } }
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
    <div className="space-y-6 print-container">
      <div className="flex flex-row items-center justify-between no-print">
        <div className="flex flex-col gap-2">
          <h2 className="text-3xl font-bold tracking-tight text-primary">Financial Analytics</h2>
          <p className="text-muted-foreground">Admin-only reporting, performance rollups, and financial statements.</p>
        </div>
        <Button onClick={handlePrint} variant="outline" className="gap-2">
          <Printer className="h-4 w-4" /> Print / PDF
        </Button>
      </div>

      {/* DASHBOARD CARDS */}
      <motion.div variants={containerVars} initial="hidden" animate="show" className="grid md:grid-cols-3 gap-6 no-print">
        <motion.div variants={itemVars}>
          <Card className="hover-lift">
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">Cash in Hand</CardTitle>
              <DollarSign className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold text-emerald-600">
                {(dashboard?.cash_in_hand || 0).toFixed(2)}
              </div>
              <p className="text-xs text-muted-foreground mt-1">Live physical cash & bank balance</p>
            </CardContent>
          </Card>
        </motion.div>
        <motion.div variants={itemVars}>
          <Card className="hover-lift">
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">Monthly Sales</CardTitle>
              <Activity className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">
                {(dashboard?.monthly_sales || 0).toFixed(2)}
              </div>
              <p className="text-xs text-muted-foreground mt-1">Sales revenue for current month</p>
            </CardContent>
          </Card>
        </motion.div>
        <motion.div variants={itemVars}>
          <Card className="hover-lift">
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">All-Time Net Profit</CardTitle>
              <TrendingUp className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className={`text-2xl font-bold ${(dashboard?.net_profit || 0) >= 0 ? 'text-emerald-600' : 'text-destructive'}`}>
                {(dashboard?.net_profit || 0).toFixed(2)}
              </div>
              <p className="text-xs text-muted-foreground mt-1">Total bottom-line after COGS & Expenses</p>
            </CardContent>
          </Card>
        </motion.div>
      </motion.div>

      <Tabs defaultValue="pl" className="w-full">
        <TabsList className="mb-4 no-print">
          <TabsTrigger value="pl">Income Statement</TabsTrigger>
          <TabsTrigger value="bs">Balance Sheet</TabsTrigger>
          <TabsTrigger value="assets">Fixed Assets</TabsTrigger>
        </TabsList>

        {/* INCOME STATEMENT */}
        <TabsContent value="pl">
          <Card className="print-container">
            <CardHeader className="text-center">
              <CardTitle className="text-2xl">Income Statement (P&L)</CardTitle>
              <CardDescription>Generated on {new Date().toLocaleDateString()}</CardDescription>
            </CardHeader>
            <CardContent className="max-w-3xl mx-auto space-y-6">
              {income && (
                <div className="space-y-4">
                  <div className="flex justify-between text-lg">
                    <span>Sales Revenue</span>
                    <span>{income.revenue.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between text-destructive">
                    <span>Cost of Goods Sold (COGS)</span>
                    <span>({income.cogs.toFixed(2)})</span>
                  </div>
                  <div className="flex justify-between font-bold text-xl border-t border-b py-2">
                    <span>Gross Profit</span>
                    <span>{income.gross_profit.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between text-destructive">
                    <span>Operating Expenses</span>
                    <span>({income.operating_expenses.toFixed(2)})</span>
                  </div>
                  <div className="flex justify-between font-bold text-2xl border-b-4 border-double py-2 mt-4 text-primary">
                    <span>Net Profit</span>
                    <span>{income.net_profit.toFixed(2)}</span>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* BALANCE SHEET */}
        <TabsContent value="bs">
          <Card className="print-container">
            <CardHeader className="text-center">
              <CardTitle className="text-2xl">Balance Sheet</CardTitle>
              <CardDescription>Generated on {new Date().toLocaleDateString()}</CardDescription>
            </CardHeader>
            <CardContent className="max-w-4xl mx-auto">
              {balance && (
                <div className="grid md:grid-cols-2 gap-12">
                  <div className="space-y-4">
                    <h3 className="font-bold text-xl border-b pb-2">Assets</h3>
                    <div className="space-y-2">
                      <div className="flex justify-between"><span>Cash / Bank</span><span>{balance.cash.toFixed(2)}</span></div>
                      <div className="flex justify-between"><span>Accounts Receivable</span><span>{balance.accounts_receivable.toFixed(2)}</span></div>
                      <div className="flex justify-between"><span>Inventory Value (Cost)</span><span>{balance.inventory_value.toFixed(2)}</span></div>
                      <div className="flex justify-between text-muted-foreground text-sm"><span>Fixed Assets (Depreciated)</span><span>{balance.fixed_assets.toFixed(2)}</span></div>
                      <div className="flex justify-between font-bold pt-4 border-t">
                        <span>Total Assets</span>
                        <span>{(balance.cash + balance.accounts_receivable + balance.inventory_value + balance.fixed_assets).toFixed(2)}</span>
                      </div>
                    </div>
                  </div>
                  <div className="space-y-4">
                    <h3 className="font-bold text-xl border-b pb-2">Liabilities & Equity</h3>
                    <div className="space-y-2">
                      <div className="flex justify-between"><span>Accounts Payable</span><span>{balance.accounts_payable.toFixed(2)}</span></div>
                      <div className="flex justify-between text-muted-foreground mt-8"><span>Retained Earnings (Net Profit)</span><span>{income?.net_profit.toFixed(2)}</span></div>
                    </div>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* FIXED ASSETS */}
        <TabsContent value="assets" className="no-print">
          <div className="grid md:grid-cols-3 gap-6">
            <Card className="md:col-span-1 h-fit">
              <CardHeader>
                <CardTitle>Register Asset</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <label className="text-sm font-medium">Asset Name</label>
                  <Input value={assetName} onChange={(e) => setAssetName(e.target.value)} placeholder="e.g. Delivery Truck" />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">Value (Cost)</label>
                  <Input type="number" value={assetValue} onChange={(e) => setAssetValue(parseFloat(e.target.value) || "")} />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">Depreciation Rate (%)</label>
                  <Input type="number" value={depRate} onChange={(e) => setDepRate(parseFloat(e.target.value) || "")} placeholder="e.g. 10" />
                </div>
                <Button className="w-full" onClick={handleRegisterAsset}>Save Asset</Button>
              </CardContent>
            </Card>

            <Card className="md:col-span-2">
              <CardHeader>
                <CardTitle>Asset Register</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="rounded-md border">
                  <Table>
                    <TableHeader className="bg-muted/50">
                      <TableRow>
                        <TableHead>Asset Name</TableHead>
                        <TableHead className="text-right">Cost Value</TableHead>
                        <TableHead className="text-right">Depreciation Rate</TableHead>
                        <TableHead className="text-right">Current Book Value</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {assets.map((a, idx) => {
                        const bookValue = a.value * (1 - (a.depreciation_rate / 100));
                        return (
                          <TableRow key={idx}>
                            <TableCell className="font-medium">{a.name}</TableCell>
                            <TableCell className="text-right">{a.value.toFixed(2)}</TableCell>
                            <TableCell className="text-right">{a.depreciation_rate}%</TableCell>
                            <TableCell className="text-right font-semibold text-primary">{bookValue.toFixed(2)}</TableCell>
                          </TableRow>
                        );
                      })}
                      {assets.length === 0 && (
                        <TableRow>
                          <TableCell colSpan={4} className="text-center py-6 text-muted-foreground">No fixed assets registered.</TableCell>
                        </TableRow>
                      )}
                    </TableBody>
                  </Table>
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
