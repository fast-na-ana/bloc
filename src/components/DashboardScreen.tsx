import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { DollarSign, CheckSquare, Package, Receipt, CreditCard, ArrowUpRight, ShoppingBag } from "lucide-react";

export function DashboardScreen() {
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
      {/* 1. Performance Summary (Span 5, Row 1-2) */}
      <motion.div variants={itemVars} className="col-span-12 xl:col-span-5 row-span-2 glass-widget flex flex-col p-6 hover-lift">
        <h2 className="text-xl font-semibold text-on-surface mb-2">Performance Summary</h2>
        <div className="flex items-center gap-4 mb-4">
          <div className="flex items-center gap-1.5"><div className="w-2.5 h-2.5 rounded-full bg-secondary"></div><span className="text-sm">Today's Sales</span></div>
          <div className="flex items-center gap-1.5"><div className="w-2.5 h-2.5 rounded-full bg-[#c5c7c8]"></div><span className="text-sm">Today's Purchases</span></div>
        </div>
        
        {/* Abstract Chart */}
        <div className="flex-1 rounded-xl bg-white/20 border border-white/30 relative flex items-end p-4 mb-4 min-h-[160px]">
          <svg className="w-full h-full absolute inset-0 text-secondary opacity-60 drop-shadow-md" preserveAspectRatio="none" viewBox="0 0 100 40">
            <path d="M0 30 Q 20 10, 40 25 T 80 15 T 100 5 L 100 40 L 0 40 Z" fill="url(#chart-gradient)" opacity="0.2"></path>
            <path d="M0 30 Q 20 10, 40 25 T 80 15 T 100 5" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="2"></path>
            <path d="M0 35 Q 20 20, 40 30 T 80 25 T 100 20" fill="none" stroke="#c5c7c8" strokeLinecap="round" strokeWidth="1.5"></path>
            <defs>
              <linearGradient id="chart-gradient" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="currentColor"></stop>
                <stop offset="100%" stopColor="currentColor" stopOpacity="0"></stop>
              </linearGradient>
            </defs>
          </svg>
        </div>
        
        <div className="flex justify-between items-end mt-auto">
          <span className="text-xs text-on-surface-variant font-medium uppercase tracking-wider">Last 7 days</span>
          <div className="text-right">
            <div className="flex items-center justify-end gap-1 text-secondary font-bold text-xl">
              +15% <ArrowUpRight className="h-5 w-5" />
            </div>
            <span className="text-xs text-on-surface-variant font-medium">Sales from yesterday</span>
          </div>
        </div>
      </motion.div>

      {/* 2. Welcome Back (Span 4, Row 1) */}
      <motion.div variants={itemVars} className="col-span-12 xl:col-span-4 row-span-1 glass-widget flex flex-col p-6 hover-lift relative overflow-hidden">
        {/* Decorative Blob */}
        <div className="absolute -top-6 -right-6 w-24 h-24 bg-white/40 backdrop-blur-2xl rounded-full shadow-[inset_0_-2px_10px_rgba(0,0,0,0.1),0_10px_30px_rgba(0,0,0,0.05)] border border-white/60 pointer-events-none"></div>

        <div className="flex items-center gap-4 mb-6 z-10">
          <div className="w-12 h-12 rounded-full bg-[#c5c7c8]/30 flex items-center justify-center">
             <div className="w-10 h-10 rounded-full bg-white flex items-center justify-center border border-white/50">
               <DollarSign className="text-primary h-5 w-5" />
             </div>
          </div>
          <div>
            <h2 className="text-xl font-semibold text-on-surface leading-tight">Welcome Back,</h2>
            <p className="text-on-surface-variant text-sm font-medium">User (Admin)</p>
          </div>
        </div>

        <div className="space-y-3 mt-2 flex-1 flex flex-col justify-end z-10">
          <div className="flex justify-between text-sm">
            <span className="text-on-surface-variant">Cash In Hand:</span>
            <span className="font-semibold text-on-surface">${cashOnHand.toFixed(2)}</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-on-surface-variant">Today's Sales:</span>
            <span className="font-semibold text-on-surface">${sales.toFixed(2)}</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-on-surface-variant">Pending Reminders:</span>
            <span className="font-semibold text-on-surface">5</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-on-surface-variant">Low Stock Alerts:</span>
            <span className="font-semibold text-error">10 items</span>
          </div>
        </div>
      </motion.div>

      {/* 3. Top 5 Overdue Payers (Span 3, Row 1) */}
      <motion.div variants={itemVars} className="col-span-12 xl:col-span-3 row-span-1 glass-widget flex flex-col p-6 hover-lift bg-gradient-to-br from-tertiary-container/50 to-error-container/20 border-error/10">
        <h2 className="text-xl font-semibold text-on-surface mb-4">Top 5 Overdue Payers</h2>
        <div className="space-y-3">
          <div className="flex justify-between text-sm border-b border-error/10 pb-2">
            <span className="text-error font-medium">ABC Co.</span>
            <span className="text-error/70">25 days</span>
          </div>
          <div className="flex justify-between text-sm border-b border-error/10 pb-2">
            <span className="text-on-surface-variant font-medium">J. Smith</span>
            <span className="text-on-surface-variant/70">18 days</span>
          </div>
          <div className="flex justify-between text-sm border-b border-error/10 pb-2">
            <span className="text-on-surface-variant font-medium">Acme Corp</span>
            <span className="text-on-surface-variant/70">12 days</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-on-surface-variant font-medium">D. Wayne</span>
            <span className="text-on-surface-variant/70">5 days</span>
          </div>
        </div>
      </motion.div>

      {/* 4. Low Stock Items (Span 5, Row 3) */}
      <motion.div variants={itemVars} className="col-span-12 xl:col-span-5 row-span-1 glass-widget flex flex-col p-6 hover-lift">
        <h2 className="text-xl font-semibold text-on-surface mb-4 flex items-center gap-2">
          <Package className="h-5 w-5 text-secondary" />
          Low Stock Items
        </h2>
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <div className="w-2 h-2 rounded-full bg-secondary"></div>
            <span className="text-sm font-medium text-on-surface">Item A <span className="text-secondary">(Qty 3)</span></span>
          </div>
          <div className="flex items-center gap-3">
            <div className="w-2 h-2 rounded-full bg-secondary"></div>
            <span className="text-sm font-medium text-on-surface">Item B <span className="text-secondary">(Qty 1)</span></span>
          </div>
        </div>
      </motion.div>

      {/* 5. Reminder & Promise Log (Span 4, Row 2-3) */}
      <motion.div variants={itemVars} className="col-span-12 xl:col-span-4 row-span-2 glass-widget flex flex-col p-6 hover-lift">
        <h2 className="text-xl font-semibold text-on-surface mb-4">Reminder & Promise Log</h2>
        <div className="space-y-3 flex-1 overflow-y-auto pr-2">
          <div className="flex items-start gap-3 p-3 rounded-xl bg-white/40 border border-white/60 shadow-sm">
             <CheckSquare className="h-5 w-5 text-secondary mt-0.5 shrink-0" />
             <span className="text-sm font-medium text-on-surface">Verify TID #987 from Bank Deposit</span>
          </div>
          <div className="flex items-start gap-3 p-3 rounded-xl hover:bg-white/20 transition-colors border border-transparent">
             <div className="h-5 w-5 rounded border-2 border-on-surface-variant/30 mt-0.5 shrink-0"></div>
             <span className="text-sm text-on-surface-variant">Call Customer Doe re: Dues (30 Days Past)</span>
          </div>
          <div className="flex flex-col gap-2 p-3 rounded-xl hover:bg-white/20 transition-colors border border-transparent">
             <div className="flex items-start gap-3">
               <div className="h-5 w-5 rounded border-2 border-on-surface-variant/30 mt-0.5 shrink-0"></div>
               <span className="text-sm text-on-surface-variant">Record Verbal Payment Promise (Smith, $500)</span>
             </div>
          </div>
        </div>
      </motion.div>

      {/* 6. Recent Cash Transactions (Span 3, Row 2) */}
      <motion.div variants={itemVars} className="col-span-12 xl:col-span-3 row-span-1 glass-widget flex flex-col p-6 hover-lift">
        <h2 className="text-xl font-semibold text-on-surface mb-4">Recent Cash Transactions</h2>
        <div className="space-y-4">
          <div>
            <p className="text-sm font-medium text-on-surface">Sale Invoice #123</p>
            <p className="text-sm text-secondary font-semibold">(+$500.00)</p>
          </div>
          <div>
            <p className="text-sm font-medium text-on-surface">Expense Entry #9</p>
            <p className="text-sm text-error font-semibold">(-$150.00)</p>
          </div>
        </div>
      </motion.div>

      {/* 7. Cash Balance (Span 3, Row 3) */}
      <motion.div variants={itemVars} className="col-span-12 xl:col-span-3 row-span-1 glass-widget flex flex-col items-center justify-center p-6 hover-lift border-secondary/20">
        <div className="flex items-center gap-2 mb-2 text-secondary">
          <DollarSign className="h-5 w-5" />
          <span className="font-medium text-sm">Cash Balance</span>
        </div>
        <h1 className="text-4xl font-bold tracking-tight text-on-surface">${cashOnHand.toLocaleString(undefined, {minimumFractionDigits: 2})}</h1>
        <p className="text-xs text-on-surface-variant mt-2">Local Vault Account</p>
      </motion.div>

      {/* 8. Quick Entry Bar (Span 12, Row 4) */}
      <motion.div variants={itemVars} className="col-span-12 row-span-1 glass-widget flex items-center justify-between p-4 px-8 hover-lift mt-2">
        <span className="font-semibold text-on-surface">Quick Entry Bar</span>
        <div className="flex gap-8">
          <button className="flex flex-col items-center gap-1 text-secondary hover:text-secondary-container transition-colors group">
            <Receipt className="h-5 w-5 group-hover:-translate-y-1 transition-transform" />
            <span className="text-[10px] font-semibold uppercase tracking-wider">[New Sale Invoice]</span>
          </button>
          <button className="flex flex-col items-center gap-1 text-primary hover:text-primary-container transition-colors group">
            <ShoppingBag className="h-5 w-5 group-hover:-translate-y-1 transition-transform" />
            <span className="text-[10px] font-semibold uppercase tracking-wider">[New Purchase Invoice]</span>
          </button>
          <button className="flex flex-col items-center gap-1 text-secondary hover:text-secondary-container transition-colors group">
            <CreditCard className="h-5 w-5 group-hover:-translate-y-1 transition-transform" />
            <span className="text-[10px] font-semibold uppercase tracking-wider">[Record Payment]</span>
          </button>
          <button className="flex flex-col items-center gap-1 text-primary hover:text-primary-container transition-colors group">
            <Package className="h-5 w-5 group-hover:-translate-y-1 transition-transform" />
            <span className="text-[10px] font-semibold uppercase tracking-wider">[Check Inventory]</span>
          </button>
        </div>
      </motion.div>

    </motion.div>
  );
}
