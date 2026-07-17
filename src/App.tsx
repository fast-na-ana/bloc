import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Layout } from "./components/Layout";
import { DashboardScreen } from "./components/DashboardScreen";
import { InvoiceScreen } from "./components/InvoiceScreen";
import { LedgerScreen } from "./components/LedgerScreen";
import { InventoryScreen } from "./components/InventoryScreen";
import { ExpenseScreen } from "./components/ExpenseScreen";
import { ReminderScreen } from "./components/ReminderScreen";
import { PaymentScreen } from "./components/PaymentScreen";
import { AnalyticsScreen } from "./components/AnalyticsScreen";
import { LoginScreen } from "./components/LoginScreen";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./App.css";

interface User {
  id: number;
  username: string;
  role: string;
}

function App() {
  const [activeView, setActiveView] = useState("dashboard");
  const [currentUser, setCurrentUser] = useState<User | null>(null);

  if (!currentUser) {
    return (
      <ErrorBoundary>
        <LoginScreen onLoginSuccess={setCurrentUser} />
      </ErrorBoundary>
    );
  }

  const renderView = () => {
    switch (activeView) {
      case "dashboard":
        return <DashboardScreen />;
      case "invoices":
        return <InvoiceScreen />;
      case "payments":
        return <PaymentScreen />;
      case "analytics":
        return <AnalyticsScreen />;
      case "ledger":
        return <LedgerScreen currentUser={currentUser} />;
      case "inventory":
        return <InventoryScreen />;
      case "expenses":
        return <ExpenseScreen />;
      case "reminders":
        return <ReminderScreen />;
      default:
        return <DashboardScreen />;
    }
  };

  return (
    <ErrorBoundary>
      <Layout activeView={activeView} onNavigate={setActiveView} currentUser={currentUser} onLogout={() => setCurrentUser(null)}>
        <AnimatePresence mode="wait">
          <motion.div
            key={activeView}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.2, ease: "easeOut" }}
            className="w-full h-full"
          >
            {renderView()}
          </motion.div>
        </AnimatePresence>
      </Layout>
    </ErrorBoundary>
  );
}

export default App;
