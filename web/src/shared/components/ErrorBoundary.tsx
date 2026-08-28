import React from "react";

interface Props {
  children: React.ReactNode;
  fallbackTitle?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("[ErrorBoundary]", error, info.componentStack);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col gap-4 rounded-xl border border-red-200 bg-red-50 px-6 py-8 text-center dark:border-red-900 dark:bg-red-950/30">
          <div className="flex flex-col items-center gap-2">
            <span className="material-symbols-outlined text-[32px] text-red-500">error</span>
            <h2 className="text-lg font-semibold text-red-700 dark:text-red-300">
              {this.props.fallbackTitle ?? "Something went wrong"}
            </h2>
            <p className="max-w-md text-sm text-red-600 dark:text-red-400">
              {this.state.error?.message ?? "An unexpected error occurred."}
            </p>
            <p className="text-xs text-zinc-500">
              Try reloading the page. If the problem persists, rebuild the dashboard (
              <code className="rounded bg-zinc-200 px-1 dark:bg-zinc-800">pnpm build</code>) and
              restart the server.
            </p>
          </div>
          <div className="flex justify-center gap-2">
            <button
              onClick={this.handleReset}
              className="rounded-lg border border-zinc-300 bg-white px-4 py-2 text-sm font-medium hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:hover:bg-zinc-800"
            >
              Try again
            </button>
            <button
              onClick={this.handleReload}
              className="rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700"
            >
              Reload page
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

export default ErrorBoundary;
