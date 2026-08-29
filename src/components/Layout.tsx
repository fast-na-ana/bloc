import { ReactNode, useState, useRef, useEffect } from "react";
import { Search, Bell, Grid, Receipt, ShoppingBag, Package, Users, TrendingUp, DollarSign, Settings, LogOut, X, PanelLeftClose, PanelLeftOpen } from "lucide-react";

interface LayoutProps {
  children: ReactNode;
  activeView: string;
  onNavigate: (view: string) => void;
  currentUser: { id: number; username: string; role: string };
  onLogout: () => void;
}

export function Layout({ children, activeView, onNavigate, currentUser, onLogout }: LayoutProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const searchRef = useRef<HTMLDivElement>(null);

  const navItemsGeneral = [
    { id: "dashboard", label: "Dashboard", icon: Grid, adminOnly: false },
    { id: "invoices", label: "Invoicing", icon: Receipt, adminOnly: false },
    { id: "payments", label: "Payments", icon: ShoppingBag, adminOnly: true },
    { id: "inventory", label: "Inventory", icon: Package, adminOnly: true },
    { id: "contacts", label: "Customers & Suppliers", icon: Users, adminOnly: false },
    { id: "analytics", label: "Reports", icon: TrendingUp, adminOnly: true },
  ];

  const navItemsOthers = [
    { id: "expenses", label: "Expenses", icon: DollarSign, adminOnly: true },
    { id: "reminders", label: "Reminders & Log", icon: Bell, adminOnly: false },
    { id: "settings", label: "Settings (Admin)", icon: Settings, adminOnly: true },
  ];

  const allSearchableItems = [...navItemsGeneral, ...navItemsOthers];

  const filteredSearchItems = searchQuery.trim() === "" 
    ? [] 
    : allSearchableItems.filter(item => 
        item.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
        item.id.toLowerCase().includes(searchQuery.toLowerCase())
      );

  // Close search popover when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (searchRef.current && !searchRef.current.contains(event.target as Node)) {
        setIsSearchOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleNavigate = (item: any) => {
    if (item.adminOnly && currentUser.role !== "admin") {
      alert("Access Denied: You do not have permission to access Admin-only routes.");
      return;
    }
    onNavigate(item.id);
    setIsSearchOpen(false);
    setSearchQuery("");
  };

  const renderNavItem = (item: any) => {
    const Icon = item.icon;
    const isActive = activeView === item.id;
    const isLocked = item.adminOnly && currentUser.role !== "admin";

    if ((item.id === "analytics" || item.id === "settings") && currentUser.role !== "admin") return null;

    if (isCollapsed) {
      return (
        <button
          key={item.id}
          onClick={() => handleNavigate(item)}
          title={`${item.label}${isLocked ? " (Admin)" : ""}`}
          className={`flex items-center justify-center p-3 rounded-2xl transition-all duration-300 text-sm font-semibold group w-full cursor-pointer ${
            isActive ? "bg-blue-600 text-white shadow-md shadow-blue-600/25" : "text-slate-700 hover:text-blue-700 hover:bg-blue-50/80"
          } ${isLocked ? "opacity-50 cursor-not-allowed" : ""}`}
        >
          <Icon className={`h-5 w-5 ${isActive ? "text-white" : "text-slate-500 group-hover:text-blue-600"}`} />
        </button>
      );
    }

    if (isActive) {
      return (
        <button
          key={item.id}
          onClick={() => handleNavigate(item)}
          className="flex items-center gap-3 px-4 py-3 bg-blue-600 text-white rounded-full shadow-md shadow-blue-600/25 transition-all duration-300 text-sm font-semibold group w-full cursor-pointer"
        >
          <Icon className="h-5 w-5 text-white shrink-0" />
          <span className="truncate">{item.label} {isLocked && "(Admin)"}</span>
        </button>
      );
    }

    return (
      <button
        key={item.id}
        onClick={() => handleNavigate(item)}
        className={`flex items-center gap-3 px-4 py-3 text-slate-700 hover:text-blue-700 hover:bg-blue-50/80 rounded-full transition-all duration-200 text-sm font-medium group w-full cursor-pointer ${isLocked ? "opacity-50 cursor-not-allowed" : ""}`}
      >
        <Icon className="h-5 w-5 text-slate-500 group-hover:text-blue-600 transition-colors shrink-0" />
        <span className="truncate">{item.label} {isLocked && "(Admin)"}</span>
      </button>
    );
  };

  return (
    <div className="flex h-screen w-full relative">
      <div className="fixed inset-0 z-[-1] aura-bg">
        <div className="aura-blob aura-blob-1"></div>
        <div className="aura-blob aura-blob-2"></div>
        <div className="aura-blob aura-blob-3"></div>
      </div>

      {/* SideNavBar */}
      <nav className={`fixed left-0 top-0 z-20 flex flex-col bg-white/85 backdrop-blur-2xl rounded-2xl m-6 h-[calc(100vh-3rem)] border border-white/90 shadow-[0_12px_40px_rgba(15,23,42,0.1)] overflow-y-auto overflow-x-hidden scrollbar-hide transition-all duration-300 ${
        isCollapsed ? "w-[76px]" : "w-[260px]"
      }`}>
        {/* Brand Logo & Collapse Toggle */}
        <div className={`p-6 pb-4 flex items-center ${isCollapsed ? "justify-center" : "justify-between"}`}>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-blue-600 flex items-center justify-center shadow-md shadow-blue-600/30 shrink-0">
              <Grid className="text-white h-5 w-5" />
            </div>
            {!isCollapsed && (
              <div className="flex flex-col">
                <span className="text-xl font-bold text-slate-900 leading-tight">LocalDesk</span>
                <span className="text-xs text-slate-500 font-medium leading-tight">Business Ops</span>
              </div>
            )}
          </div>
        </div>

        {/* Navigation Links */}
        <div className={`flex flex-col gap-1 py-2 ${isCollapsed ? "px-2" : "px-4"}`}>
          {!isCollapsed && <p className="text-[11px] font-bold text-slate-400 mt-2 mb-1 uppercase tracking-widest pl-4">GENERAL</p>}
          {navItemsGeneral.map(renderNavItem)}

          {!isCollapsed && <p className="text-[11px] font-bold text-slate-400 mt-6 mb-1 uppercase tracking-widest pl-4">OTHERS</p>}
          {navItemsOthers.map(renderNavItem)}
        </div>

        {/* Logout */}
        <div className={`mt-auto ${isCollapsed ? "p-2" : "p-4"}`}>
          {isCollapsed ? (
            <button
              onClick={onLogout}
              title="Logout"
              className="flex items-center justify-center p-3 text-slate-700 hover:bg-rose-50 hover:text-rose-600 rounded-2xl transition-all duration-200 text-sm font-medium group w-full cursor-pointer"
            >
              <LogOut className="h-5 w-5 text-slate-500 group-hover:text-rose-600 transition-colors" />
            </button>
          ) : (
            <button
              onClick={onLogout}
              className="flex items-center gap-3 px-4 py-3 text-slate-700 hover:bg-rose-50 hover:text-rose-600 rounded-full transition-all duration-200 text-sm font-medium group w-full cursor-pointer"
            >
              <LogOut className="h-5 w-5 text-slate-500 group-hover:text-rose-600 transition-colors shrink-0" />
              <span>Logout</span>
            </button>
          )}
        </div>
      </nav>

      {/* Main Content Area */}
      <main className={`flex-1 flex flex-col h-screen overflow-hidden relative z-10 transition-all duration-300 ${
        isCollapsed ? "ml-[124px] w-[calc(100%-124px)]" : "ml-[308px] w-[calc(100%-308px)]"
      }`}>
        {/* Top Header Bar */}
        <header className="flex justify-between items-center w-full px-8 py-5 bg-transparent z-30 flex-shrink-0 relative">
          <div className="flex items-center gap-3">
            <button
              onClick={() => setIsCollapsed(!isCollapsed)}
              className="p-2.5 rounded-full bg-white/90 border border-white/90 text-slate-600 hover:text-blue-600 hover:bg-white transition-all shadow-xs active:scale-95 cursor-pointer flex items-center justify-center"
              title={isCollapsed ? "Expand Sidebar" : "Collapse Sidebar"}
            >
              {isCollapsed ? <PanelLeftOpen className="h-4 w-4" /> : <PanelLeftClose className="h-4 w-4" />}
            </button>
            <span className="text-base font-extrabold text-slate-900 capitalize tracking-tight">
              {activeView.replace("_", " ")}
            </span>
          </div>
          
          <div className="flex items-center gap-4">
            {/* Search Bar with Interactive Dropdown */}
            <div className="relative w-80" ref={searchRef}>
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 text-slate-400 h-4 w-4 pointer-events-none" />
              <input
                className="w-full bg-white/90 backdrop-blur-md border border-white/90 text-slate-900 placeholder:text-slate-400 text-sm rounded-full py-2.5 pl-11 pr-9 focus:outline-none focus:ring-2 focus:ring-blue-500/40 transition-all shadow-xs"
                placeholder="Search views, invoices, items..."
                type="text"
                value={searchQuery}
                onFocus={() => setIsSearchOpen(true)}
                onChange={(e) => {
                  setSearchQuery(e.target.value);
                  setIsSearchOpen(true);
                }}
              />
              {searchQuery && (
                <button 
                  onClick={() => { setSearchQuery(""); setIsSearchOpen(false); }}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              )}

              {/* Quick Search Results Popover */}
              {isSearchOpen && filteredSearchItems.length > 0 && (
                <div className="absolute top-full left-0 right-0 mt-2 bg-white/95 backdrop-blur-xl border border-slate-200/80 rounded-2xl shadow-xl z-50 p-2 overflow-hidden max-h-64 overflow-y-auto">
                  <div className="text-[10px] font-bold text-slate-400 px-3 py-1 uppercase tracking-wider">
                    Matching Quick Views ({filteredSearchItems.length})
                  </div>
                  {filteredSearchItems.map((item) => {
                    const Icon = item.icon;
                    return (
                      <button
                        key={item.id}
                        onClick={() => handleNavigate(item)}
                        className="w-full flex items-center gap-3 px-3 py-2 text-sm text-slate-800 hover:bg-blue-50 hover:text-blue-700 rounded-xl transition-colors text-left font-medium cursor-pointer"
                      >
                        <Icon className="h-4 w-4 text-blue-600" />
                        <span>{item.label}</span>
                        {item.adminOnly && <span className="ml-auto text-[10px] bg-slate-100 text-slate-600 px-2 py-0.5 rounded-md font-semibold">Admin</span>}
                      </button>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Notification Bell Button */}
            <button 
              onClick={() => onNavigate("reminders")}
              title="View Reminders & Notifications"
              className="relative w-10 h-10 rounded-full bg-white/90 backdrop-blur-md border border-white/90 flex items-center justify-center text-slate-700 hover:bg-white hover:text-blue-600 transition-all shadow-xs active:scale-95 cursor-pointer"
            >
              <Bell className="h-4 w-4" />
              <span className="absolute -top-0.5 -right-0.5 w-4 h-4 bg-rose-500 text-white text-[10px] font-bold rounded-full flex items-center justify-center shadow-xs">
                5
              </span>
            </button>

            {/* User Account Button */}
            <button 
              onClick={() => {
                if (currentUser.role === "admin") {
                  onNavigate("settings");
                } else {
                  alert(`Logged in as: ${currentUser.username} (${currentUser.role.toUpperCase()})`);
                }
              }}
              title="Account Settings"
              className="h-10 px-4 rounded-full bg-white/90 border border-white/90 flex items-center justify-center text-slate-800 hover:bg-white hover:border-blue-200 transition-all shadow-xs text-sm font-semibold gap-2 cursor-pointer"
            >
              <Users className="h-4 w-4 text-blue-600" />
              <span>{currentUser.username}</span>
            </button>
          </div>
        </header>

        {/* Dynamic Canvas */}
        <div className="flex-1 overflow-y-auto px-8 pb-8 pt-2 relative">
          {children}
        </div>
      </main>
    </div>
  );
}
