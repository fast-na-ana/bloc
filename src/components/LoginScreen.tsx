import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Lock, User } from "lucide-react";

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

  if (isLoading) return <div className="h-screen w-screen flex items-center justify-center bg-background"><p>Connecting securely...</p></div>;

  return (
    <div className="h-screen w-screen flex items-center justify-center bg-muted/20">
      <Card className="w-full max-w-md shadow-lg border-primary/10">
        <CardHeader className="text-center space-y-2">
          <div className="mx-auto bg-primary/10 p-3 rounded-full w-fit mb-2">
            <Lock className="w-6 h-6 text-primary" />
          </div>
          <CardTitle className="text-2xl font-bold tracking-tight">
            {isSetupMode ? "Setup System Admin" : "Welcome back"}
          </CardTitle>
          <CardDescription>
            {isSetupMode ? "Initialize your local business environment." : "Enter your credentials to access your terminal."}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <label className="text-sm font-medium">Username</label>
            <div className="relative">
              <User className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input className="pl-9" placeholder="Enter username" value={username} onChange={e => setUsername(e.target.value)} />
            </div>
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Password</label>
            <div className="relative">
              <Lock className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input className="pl-9" type="password" placeholder="••••••••" value={password} onChange={e => setPassword(e.target.value)} onKeyDown={(e) => {
                if (e.key === "Enter") {
                  isSetupMode ? handleSetup() : handleLogin();
                }
              }} />
            </div>
          </div>
          
          <Button className="w-full mt-6" onClick={isSetupMode ? handleSetup : handleLogin}>
            {isSetupMode ? "Initialize System" : "Secure Login"}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
