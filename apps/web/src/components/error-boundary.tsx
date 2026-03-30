"use client";

import { Component, type ReactNode } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="p-8 max-w-2xl">
          <div className="border border-red-900/20 bg-red-950/10 p-8">
            <h2 className="text-sm font-mono uppercase tracking-[0.2em] text-red-400 mb-4">
              Runtime Error
            </h2>
            <p className="text-sm font-light text-neutral-300 mb-6">
              Something went wrong rendering this view.
            </p>
            {this.state.error && (
              <pre className="text-[10px] font-mono text-red-400/70 mb-6 overflow-x-auto whitespace-pre-wrap">
                {this.state.error.message}
              </pre>
            )}
            <button
              onClick={() => this.setState({ hasError: false, error: null })}
              className="text-[10px] font-mono uppercase tracking-wider px-4 py-2 border border-white/10 text-neutral-500 hover:text-white hover:border-white/20 transition-all duration-150"
            >
              Try Again
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
