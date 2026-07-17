import { ReactNode } from "react";
import { Search, Bell, Grid, Receipt, ShoppingBag, Package, Users, TrendingUp, DollarSign, CreditCard, Settings, LogOut } from "lucide-react";

interface LayoutProps {
  children: ReactNode;
  activeView: string;
  onNavigate: (view: string) => void;
  currentUser: { id: number; username: string; role: string };
  onLogout: () => void;
}

export function Layout({ children, activeView, onNavigate, currentUser, onLogout }: LayoutProps) {
  const navItemsGeneral = [
    { id: "dashboard", label: "Dashboard", icon: Grid, adminOnly: false },
    { id: "invoices", label: "Sales", icon: Receipt, adminOnly: false },
    { id: "payments", label: "Purchases", icon: ShoppingBag, adminOnly: true },
    { id: "inventory", label: "Inventory", icon: Package, adminOnly: true },
    { id: "contacts", label: "Customers & Suppliers", icon: Users, adminOnly: false },
    { id: "analytics", label: "Reports", icon: TrendingUp, adminOnly: true },
  ];

  const navItemsOthers = [
    { id: "expenses", label: "Expenses", icon: DollarSign, adminOnly: true },
    { id: "ledger", label: "Credit Limits", icon: CreditCard, adminOnly: true },
    { id: "settings", label: "Settings (Admin)", icon: Settings, adminOnly: true },
  ];

  const handleNavigate = (item: any) => {
    if (item.adminOnly && currentUser.role !== "admin") {
      alert("Access Denied: You do not have permission to access Admin-only routes.");
      return;
    }
    onNavigate(item.id);
  };

  const renderNavItem = (item: any) => {
    const Icon = item.icon;
    const isActive = activeView === item.id;
    const isLocked = item.adminOnly && currentUser.role !== "admin";

    // Hide certain items entirely from non-admins if desired, or just gray them out
    if ((item.id === "analytics" || item.id === "settings") && currentUser.role !== "admin") return null;

    if (isActive) {
      return (
        <button
          key={item.id}
          onClick={() => handleNavigate(item)}
          className="flex items-center gap-3 px-4 py-3 bg-white/80 dark:bg-white/20 text-on-surface dark:text-inverse-on-surface rounded-full shadow-sm transition-all duration-300 scale-95 active:scale-90 text-label-lg font-label-lg group w-full"
        >
          <Icon className="h-5 w-5 text-secondary" />
          <span>{item.label} {isLocked && "(Admin)"}</span>
        </button>
      );
    }

    return (
      <button
        key={item.id}
        onClick={() => handleNavigate(item)}
        className={`flex items-center gap-3 px-4 py-3 text-on-surface-variant dark:text-surface-variant hover:bg-white/20 dark:hover:bg-white/5 rounded-full transition-all duration-300 hover:backdrop-blur-2xl active:scale-90 text-label-lg font-label-lg group w-full ${isLocked ? "opacity-50 cursor-not-allowed" : ""}`}
      >
        <Icon className="h-5 w-5 group-hover:text-primary transition-colors" />
        <span>{item.label} {isLocked && "(Admin)"}</span>
      </button>
    );
  };

  return (
    <div className="flex h-screen w-full relative">
      <div className="fixed inset-0 z-[-1] aura-bg"></div>

      {/* SideNavBar */}
      <nav className="fixed left-0 top-0 z-20 flex flex-col bg-white/10 dark:bg-black/10 backdrop-blur-3xl rounded-lg m-6 w-[260px] h-[calc(100vh-3rem)] border border-white/40 dark:border-white/10 shadow-[0px_20px_50px_rgba(0,0,0,0.05)] overflow-y-auto overflow-x-hidden scrollbar-hide">
        {/* Brand Logo */}
        <div className="p-8 pb-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-gradient-to-tr from-tertiary-fixed-dim to-secondary-container flex items-center justify-center shadow-inner">
              <Grid className="text-white h-5 w-5" />
            </div>
            <div className="flex flex-col">
              <span className="text-xl font-bold text-on-surface leading-tight">LocalDesk</span>
              <span className="text-xs text-on-surface-variant leading-tight">Business Ops</span>
            </div>
          </div>
        </div>

        {/* Navigation Links */}
        <div className="flex flex-col gap-1 px-4 py-2">
          <p className="text-[11px] font-medium text-on-surface-variant mt-2 mb-1 uppercase tracking-widest pl-4">GENERAL</p>
          {navItemsGeneral.map(renderNavItem)}

          <p className="text-[11px] font-medium text-on-surface-variant mt-6 mb-1 uppercase tracking-widest pl-4">OTHERS</p>
          {navItemsOthers.map(renderNavItem)}
        </div>

        {/* Logout */}
        <div className="p-4 mt-auto">
          <button
            onClick={onLogout}
            className="flex items-center gap-3 px-4 py-3 text-on-surface-variant dark:text-surface-variant hover:bg-white/20 dark:hover:bg-white/5 rounded-full transition-all duration-300 hover:backdrop-blur-2xl active:scale-90 text-label-lg font-label-lg group w-full"
          >
            <LogOut className="h-5 w-5 group-hover:text-error transition-colors" />
            <span>Logout</span>
          </button>
        </div>
      </nav>

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col ml-[280px] w-[calc(100%-280px)] h-screen overflow-hidden relative z-10">
        {/* TopAppBar */}
        <header className="flex justify-between items-center w-full px-12 py-6 bg-transparent z-20 flex-shrink-0">
          
          <div className="flex gap-2 bg-white/30 backdrop-blur-md rounded-full p-1 shadow-sm border border-white/40">
            <button className="px-4 py-1.5 rounded-full text-sm font-medium bg-white text-on-surface shadow-sm">Local Accounts</button>
            <button className="px-4 py-1.5 rounded-full text-sm font-medium text-on-surface-variant hover:bg-white/50 transition-colors">Cashflow</button>
            <button className="px-4 py-1.5 rounded-full text-sm font-medium text-on-surface-variant hover:bg-white/50 transition-colors">Inventory</button>
          </div>

          <div className="flex items-center gap-6">
            {/* Search Bar */}
            <div className="relative w-80">
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 text-on-surface-variant h-4 w-4" />
              <input
                className="w-full bg-white/40 backdrop-blur-md border border-white/40 text-on-surface placeholder:text-on-surface-variant text-sm rounded-full py-2.5 pl-12 pr-4 focus:outline-none focus:ring-2 focus:ring-secondary/30 focus:border-secondary/50 transition-all shadow-[0_4px_12px_rgba(0,0,0,0.02)]"
                placeholder="Search invoices, items, customers..."
                type="text"
              />
            </div>

            <button className="w-10 h-10 rounded-full bg-white/40 backdrop-blur-md border border-white/40 flex items-center justify-center text-secondary hover:bg-white/60 transition-all shadow-sm active:scale-95">
              <Bell className="h-4 w-4" />
            </button>

            <button className="h-10 px-4 rounded-full bg-secondary-container/30 border border-secondary-container/50 flex items-center justify-center text-on-surface-variant hover:bg-secondary-container/50 transition-all shadow-sm text-sm font-medium gap-2">
              <Users className="h-4 w-4 text-secondary" />
              Account: {currentUser.username}
            </button>
          </div>
        </header>

        {/* Dynamic Canvas */}
        <div className="flex-1 overflow-y-auto px-12 pb-12 pt-4 relative">
          {children}
        </div>
      </main>
    </div>
  );
}

