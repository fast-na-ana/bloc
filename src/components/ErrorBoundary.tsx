import { Component, ErrorInfo, ReactNode } from "react";
import { AlertTriangle, RefreshCcw } from "lucide-react";
import { Button } from "./ui/button";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error:", error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div className="h-screen w-screen flex flex-col items-center justify-center bg-background p-4 text-center">
          <div className="bg-destructive/10 p-4 rounded-full mb-6 text-destructive">
            <AlertTriangle className="w-12 h-12" />
          </div>
          <h2 className="text-2xl font-bold tracking-tight mb-2">Something went wrong</h2>
          <p className="text-muted-foreground max-w-md mb-8">
            An unexpected error occurred in the application view. Your data is safe.
          </p>
          <div className="bg-muted p-4 rounded-md text-left w-full max-w-xl overflow-auto text-xs font-mono text-muted-foreground mb-8">
            {this.state.error?.message}
          </div>
          <Button onClick={() => window.location.reload()} className="gap-2">
            <RefreshCcw className="w-4 h-4" /> Reload Application
          </Button>
        </div>
      );
    }

    return this.props.children;
  }
}
