import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Lock, User, ShieldCheck } from "lucide-react";

interface LoginResponse {
  success: boolean;
  user: { id: number; username: string; role: string } | null;
  message: string;
}

interface LoginScreenProps {
  onLoginSuccess: (user: { id: number; username: string; role: string }) => void;
}

export function LoginScreen({ onLoginSuccess }: LoginScreenProps) {
  const [isSetupMode, setIsSetupMode] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  useEffect(() => {
    // Check if any users exist
    invoke<boolean>("check_has_users").then((hasUsers) => {
      if (!hasUsers) {
        setIsSetupMode(true);
      }
      setIsLoading(false);
    }).catch(err => {
      console.error("DB connection error: ", err);
      setIsLoading(false);
    });
  }, []);

  const handleSetup = async () => {
    if (!username || !password) return alert("Please fill all fields");
    try {
      await invoke("setup_admin", { username, passwordRaw: password });
      alert("Admin account created! Please log in.");
      setIsSetupMode(false);
      setPassword("");
    } catch (err) {
      alert(err);
    }
  };

  const handleLogin = async () => {
    if (!username || !password) return alert("Please fill all fields");
    try {
      const res = await invoke<LoginResponse>("login", { username, passwordRaw: password });
      if (res.success && res.user) {
        onLoginSuccess(res.user);
      } else {
        alert(res.message);
      }
    } catch (err) {
      alert(err);
    }
  };

  if (isLoading) {
    return (
      <div className="h-screen w-screen flex items-center justify-center relative overflow-hidden">
        <div className="fixed inset-0 z-[-1] aura-bg">
          <div className="aura-blob aura-blob-1"></div>
          <div className="aura-blob aura-blob-2"></div>
        </div>
        <p className="text-slate-700 font-extrabold text-sm relative z-10">Connecting securely...</p>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen flex items-center justify-center relative p-4 overflow-hidden">
      {/* Background canvas layer */}
      <div className="fixed inset-0 aura-bg pointer-events-none">
        <div className="aura-blob aura-blob-1"></div>
        <div className="aura-blob aura-blob-2"></div>
        <div className="aura-blob aura-blob-3"></div>
      </div>

      {/* Foreground Interactive Card */}
      <motion.div 
        initial={{ opacity: 0, y: 20, scale: 0.95 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={{ duration: 0.4, ease: "easeOut" }}
        className="w-full max-w-md glass-widget p-8 hover-lift space-y-6 bg-white/85 border-slate-200 shadow-2xl backdrop-blur-2xl relative z-10 pointer-events-auto"
      >
        <div className="text-center space-y-2 relative z-10">
          <div className="mx-auto bg-blue-50 border border-blue-100 p-3.5 rounded-2xl w-fit mb-2 shadow-xs">
            {isSetupMode ? <ShieldCheck className="w-7 h-7 text-blue-600" /> : <Lock className="w-7 h-7 text-blue-600" />}
          </div>
          <h1 className="text-2xl font-extrabold tracking-tight text-slate-900">
            {isSetupMode ? "Setup System Admin" : "Welcome Back"}
          </h1>
          <p className="text-slate-500 text-xs font-semibold">
            {isSetupMode ? "Initialize your local business environment." : "Enter your credentials to access your terminal."}
          </p>
        </div>

        <div className="space-y-4 relative z-10">
          <div className="space-y-1.5">
            <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Username</label>
            <div className="relative">
              <User className="absolute left-3 top-2.5 h-4 w-4 text-slate-400 pointer-events-none z-10" />
              <Input className="pl-9 bg-white/90 border-slate-200 text-slate-900 font-semibold focus:ring-blue-500 relative z-0" placeholder="Enter username" value={username} onChange={e => setUsername(e.target.value)} />
            </div>
          </div>
          <div className="space-y-1.5">
            <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Password</label>
            <div className="relative">
              <Lock className="absolute left-3 top-2.5 h-4 w-4 text-slate-400 pointer-events-none z-10" />
              <Input className="pl-9 bg-white/90 border-slate-200 text-slate-900 font-semibold focus:ring-blue-500 relative z-0" type="password" placeholder="••••••••" value={password} onChange={e => setPassword(e.target.value)} onKeyDown={(e) => {
                if (e.key === "Enter") {
                  isSetupMode ? handleSetup() : handleLogin();
                }
              }} />
            </div>
          </div>
          
          <Button className="w-full mt-6 bg-blue-600 hover:bg-blue-700 text-white font-bold shadow-md shadow-blue-600/20 py-2.5 cursor-pointer" onClick={isSetupMode ? handleSetup : handleLogin}>
            {isSetupMode ? "Create Administrator Account" : "Sign In to Workspace"}
          </Button>
        </div>
      </motion.div>
    </div>
  );
}
