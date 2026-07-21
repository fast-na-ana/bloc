import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { DollarSign, CheckSquare, Package, Receipt, CreditCard, ArrowUpRight, ShoppingBag } from "lucide-react";

interface DashboardScreenProps {
  onNavigate?: (view: string) => void;
}

export function DashboardScreen({ onNavigate }: DashboardScreenProps) {
  const [sales, setSales] = useState(0);
  const [cashOnHand, setCashOnHand] = useState(0);

  useEffect(() => {
    // In a real app, fetch these from the backend
    setSales(3150.00);
    setCashOnHand(12500.00);
  }, []);

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

  return (
    <motion.div 
      variants={containerVars} 
      initial="hidden" 
      animate="show"
      className="grid grid-cols-12 gap-6 h-full auto-rows-max"
    >
      {/* ROW 1: Executive Overview (Welcome Back + Cash Balance) */}
      {/* 1. Welcome Back (Span 8 on lg+, 12 on sm/md) */}
      <motion.div 
        variants={itemVars} 
        onClick={() => onNavigate?.("analytics")}
        title="Click to view Account Rollup"
        className="col-span-12 lg:col-span-8 glass-widget flex flex-col p-6 hover-lift relative overflow-hidden cursor-pointer transition-all hover:ring-2 hover:ring-blue-500/30"
      >
        {/* Decorative Blob */}
        <div className="absolute -top-6 -right-6 w-24 h-24 bg-blue-100/50 backdrop-blur-2xl rounded-full border border-blue-200/60 pointer-events-none"></div>

        <div className="flex items-center gap-4 mb-6 z-10">
          <div className="w-12 h-12 rounded-2xl bg-blue-50 border border-blue-100 flex items-center justify-center shadow-xs">
            <DollarSign className="text-blue-600 h-6 w-6" />
          </div>
          <div>
            <h2 className="text-xl font-bold text-slate-900 leading-tight">Welcome Back,</h2>
            <p className="text-slate-600 text-sm font-semibold">User (Admin)</p>
          </div>
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-2 z-10 bg-slate-50/70 p-4 rounded-xl border border-slate-100">
          <div>
            <span className="text-slate-500 text-xs font-semibold uppercase tracking-wider block">Cash In Hand</span>
            <span className="font-extrabold text-slate-900 text-base">${cashOnHand.toFixed(2)}</span>
          </div>
          <div>
            <span className="text-slate-500 text-xs font-semibold uppercase tracking-wider block">Today's Sales</span>
            <span className="font-extrabold text-slate-900 text-base">${sales.toFixed(2)}</span>
          </div>
          <div>
            <span className="text-slate-500 text-xs font-semibold uppercase tracking-wider block">Pending Reminders</span>
            <span className="font-extrabold text-slate-900 text-base">5</span>
          </div>
          <div>
            <span className="text-slate-500 text-xs font-semibold uppercase tracking-wider block">Low Stock</span>
            <span className="font-extrabold text-rose-600 text-base">10 items</span>
          </div>
        </div>
      </motion.div>

      {/* 2. Cash Balance (Span 4 on lg+, 12 on sm/md) */}
      <motion.div 
        variants={itemVars} 
        onClick={() => onNavigate?.("payments")}
        title="Click to view Purchases & Cashflow"
        className="col-span-12 lg:col-span-4 glass-widget flex flex-col items-center justify-center p-6 hover-lift border-blue-200/80 cursor-pointer transition-all hover:ring-2 hover:ring-blue-500/30"
      >
        <div className="flex items-center gap-2 mb-2 text-blue-600">
          <DollarSign className="h-5 w-5" />
          <span className="font-bold text-sm uppercase tracking-wider">Cash Balance</span>
        </div>
        <h1 className="text-4xl font-extrabold tracking-tight text-slate-900">${cashOnHand.toLocaleString(undefined, {minimumFractionDigits: 2})}</h1>
        <p className="text-xs font-semibold text-slate-500 mt-2">Local Vault Account</p>
      </motion.div>

      {/* ROW 2: Primary Analytics & Overdue Payers */}
      {/* 3. Performance Summary Chart (Span 8 on lg+, 12 on sm/md) */}
      <motion.div 
        variants={itemVars} 
        onClick={() => onNavigate?.("analytics")}
        title="Click to view Financial Reports & Analytics"
        className="col-span-12 lg:col-span-8 glass-widget flex flex-col p-6 hover-lift cursor-pointer transition-all hover:ring-2 hover:ring-blue-500/30"
      >
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-xl font-semibold text-slate-900">Performance Summary</h2>
          <span className="text-xs font-semibold text-blue-600 bg-blue-50 px-2.5 py-1 rounded-full flex items-center gap-1">
            Reports <ArrowUpRight className="h-3.5 w-3.5" />
          </span>
        </div>
        <div className="flex items-center gap-4 mb-4">
          <div className="flex items-center gap-1.5"><div className="w-2.5 h-2.5 rounded-full bg-blue-600"></div><span className="text-sm text-slate-700 font-medium">Today's Sales</span></div>
          <div className="flex items-center gap-1.5"><div className="w-2.5 h-2.5 rounded-full bg-slate-400"></div><span className="text-sm text-slate-700 font-medium">Today's Purchases</span></div>
        </div>
        
        {/* Abstract Chart */}
        <div className="flex-1 rounded-xl bg-blue-50/50 border border-blue-100 relative flex items-end p-4 mb-4 min-h-[160px]">
          <svg className="w-full h-full absolute inset-0 text-blue-600 opacity-70 drop-shadow-md" preserveAspectRatio="none" viewBox="0 0 100 40">
            <path d="M0 30 Q 20 10, 40 25 T 80 15 T 100 5 L 100 40 L 0 40 Z" fill="url(#chart-gradient)" opacity="0.25"></path>
            <path d="M0 30 Q 20 10, 40 25 T 80 15 T 100 5" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="2.5"></path>
            <path d="M0 35 Q 20 20, 40 30 T 80 25 T 100 20" fill="none" stroke="#94a3b8" strokeLinecap="round" strokeWidth="1.5"></path>
            <defs>
              <linearGradient id="chart-gradient" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="currentColor"></stop>
                <stop offset="100%" stopColor="currentColor" stopOpacity="0"></stop>
              </linearGradient>
            </defs>
          </svg>
        </div>
        
        <div className="flex justify-between items-end mt-auto">
          <span className="text-xs text-slate-500 font-bold uppercase tracking-wider">Last 7 days</span>
          <div className="text-right">
            <div className="flex items-center justify-end gap-1 text-blue-600 font-bold text-xl">
              +15% <ArrowUpRight className="h-5 w-5" />
            </div>
            <span className="text-xs text-slate-500 font-medium">Sales from yesterday</span>
          </div>
        </div>
      </motion.div>

      {/* 4. Top 5 Overdue Payers (Span 4 on lg+, 12 on sm/md) */}
      <motion.div 
        variants={itemVars} 
        onClick={() => onNavigate?.("ledger")}
        title="Click to view Credit Limits & Overdue Payers"
        className="col-span-12 lg:col-span-4 glass-widget flex flex-col p-6 hover-lift bg-gradient-to-br from-amber-50/60 to-rose-50/60 border-rose-200/60 cursor-pointer transition-all hover:ring-2 hover:ring-rose-500/30"
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-slate-900">Overdue Payers</h2>
          <span className="text-xs font-bold text-rose-600 bg-rose-100 px-2 py-0.5 rounded-md">Ledger</span>
        </div>
        <div className="space-y-3">
          <div className="flex justify-between text-sm border-b border-rose-100 pb-2">
            <span className="text-rose-700 font-bold">ABC Co.</span>
            <span className="text-rose-600/80 font-medium">25 days</span>
          </div>
          <div className="flex justify-between text-sm border-b border-rose-100 pb-2">
            <span className="text-slate-700 font-medium">J. Smith</span>
            <span className="text-slate-500">18 days</span>
          </div>
          <div className="flex justify-between text-sm border-b border-rose-100 pb-2">
            <span className="text-slate-700 font-medium">Acme Corp</span>
            <span className="text-slate-500">12 days</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-slate-700 font-medium">D. Wayne</span>
            <span className="text-slate-500">5 days</span>
          </div>
        </div>
      </motion.div>

      {/* ROW 3: Operational Monitoring (3 Equal 4-Column Cards) */}
      {/* 5. Low Stock Items (Span 4 on lg+, 6 on md, 12 on sm) */}
      <motion.div 
        variants={itemVars} 
        onClick={() => onNavigate?.("inventory")}
        title="Click to view Inventory Management"
        className="col-span-12 md:col-span-6 lg:col-span-4 glass-widget flex flex-col p-6 hover-lift cursor-pointer transition-all hover:ring-2 hover:ring-blue-500/30"
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-slate-900 flex items-center gap-2">
            <Package className="h-5 w-5 text-blue-600" />
            Low Stock Items
          </h2>
          <span className="text-xs font-bold text-blue-600 bg-blue-50 px-2.5 py-1 rounded-full">Stock</span>
        </div>
        <div className="space-y-2">
          <div className="flex items-center justify-between p-2 rounded-lg bg-slate-50 border border-slate-100">
            <div className="flex items-center gap-3">
              <div className="w-2.5 h-2.5 rounded-full bg-rose-500"></div>
              <span className="text-sm font-semibold text-slate-800">Item A</span>
            </div>
            <span className="text-xs font-bold text-rose-600 bg-rose-50 px-2 py-0.5 rounded-md">Qty 3</span>
          </div>
          <div className="flex items-center justify-between p-2 rounded-lg bg-slate-50 border border-slate-100">
            <div className="flex items-center gap-3">
              <div className="w-2.5 h-2.5 rounded-full bg-rose-500"></div>
              <span className="text-sm font-semibold text-slate-800">Item B</span>
            </div>
            <span className="text-xs font-bold text-rose-600 bg-rose-50 px-2 py-0.5 rounded-md">Qty 1</span>
          </div>
        </div>
      </motion.div>

      {/* 6. Recent Cash Transactions (Span 4 on lg+, 6 on md, 12 on sm) */}
      <motion.div 
        variants={itemVars} 
        onClick={() => onNavigate?.("invoices")}
        title="Click to view Sales & Transactions"
        className="col-span-12 md:col-span-6 lg:col-span-4 glass-widget flex flex-col p-6 hover-lift cursor-pointer transition-all hover:ring-2 hover:ring-blue-500/30"
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-slate-900">Recent Cash</h2>
          <span className="text-xs font-bold text-blue-600 bg-blue-50 px-2.5 py-1 rounded-full">Sales</span>
        </div>
        <div className="space-y-4">
          <div className="flex justify-between items-center p-2 rounded-lg bg-emerald-50/60 border border-emerald-100">
            <div>
              <p className="text-sm font-bold text-slate-800">Sale Invoice #123</p>
              <p className="text-xs text-slate-500 font-medium">Cash Sale</p>
            </div>
            <p className="text-sm text-emerald-700 font-bold">(+$500.00)</p>
          </div>
          <div className="flex justify-between items-center p-2 rounded-lg bg-rose-50/60 border border-rose-100">
            <div>
              <p className="text-sm font-bold text-slate-800">Expense Entry #9</p>
              <p className="text-xs text-slate-500 font-medium">Store Supplies</p>
            </div>
            <p className="text-sm text-rose-700 font-bold">(-$150.00)</p>
          </div>
        </div>
      </motion.div>

      {/* 7. Reminder & Promise Log (Span 4 on lg+, 12 on md/sm) */}
      <motion.div 
        variants={itemVars} 
        onClick={() => onNavigate?.("reminders")}
        title="Click to view Reminders & Log"
        className="col-span-12 md:col-span-12 lg:col-span-4 glass-widget flex flex-col p-6 hover-lift cursor-pointer transition-all hover:ring-2 hover:ring-blue-500/30"
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-slate-900">Reminder Log</h2>
          <span className="text-xs font-bold text-blue-600 bg-blue-50 px-2.5 py-1 rounded-full">View All</span>
        </div>
        <div className="space-y-2 flex-1 overflow-y-auto pr-1">
          <div className="flex items-start gap-2.5 p-2 rounded-lg bg-blue-50/70 border border-blue-100">
             <CheckSquare className="h-4 w-4 text-blue-600 mt-0.5 shrink-0" />
             <span className="text-xs font-semibold text-slate-800">Verify TID #987 from Bank Deposit</span>
          </div>
          <div className="flex items-start gap-2.5 p-2 rounded-lg bg-slate-50 border border-slate-100">
             <div className="h-4 w-4 rounded border-2 border-slate-400 mt-0.5 shrink-0"></div>
             <span className="text-xs text-slate-700 font-medium">Call Customer Doe re: Dues (30 Days Past)</span>
          </div>
        </div>
      </motion.div>

      {/* ROW 4: Quick Entry Bar (Span 12 Full Width) */}
      <motion.div variants={itemVars} className="col-span-12 glass-widget flex flex-wrap items-center justify-between p-4 px-8 hover-lift gap-4">
        <span className="font-bold text-slate-900 text-base">Quick Entry Bar</span>
        <div className="flex flex-wrap gap-6 sm:gap-8">
          <button 
            onClick={() => onNavigate?.("invoices")}
            className="flex flex-col items-center gap-1 text-blue-600 hover:text-blue-800 transition-colors group cursor-pointer"
          >
            <Receipt className="h-5 w-5 group-hover:-translate-y-1 transition-transform" />
            <span className="text-[10px] font-bold uppercase tracking-wider">[New Sale Invoice]</span>
          </button>
          <button 
            onClick={() => onNavigate?.("payments")}
            className="flex flex-col items-center gap-1 text-indigo-600 hover:text-indigo-800 transition-colors group cursor-pointer"
          >
            <ShoppingBag className="h-5 w-5 group-hover:-translate-y-1 transition-transform" />
            <span className="text-[10px] font-bold uppercase tracking-wider">[New Purchase Invoice]</span>
          </button>
          <button 
            onClick={() => onNavigate?.("payments")}
            className="flex flex-col items-center gap-1 text-blue-600 hover:text-blue-800 transition-colors group cursor-pointer"
          >
            <CreditCard className="h-5 w-5 group-hover:-translate-y-1 transition-transform" />
            <span className="text-[10px] font-bold uppercase tracking-wider">[Record Payment]</span>
          </button>
          <button 
            onClick={() => onNavigate?.("inventory")}
            className="flex flex-col items-center gap-1 text-indigo-600 hover:text-indigo-800 transition-colors group cursor-pointer"
          >
            <Package className="h-5 w-5 group-hover:-translate-y-1 transition-transform" />
            <span className="text-[10px] font-bold uppercase tracking-wider">[Check Inventory]</span>
          </button>
        </div>
      </motion.div>

    </motion.div>
  );
}
