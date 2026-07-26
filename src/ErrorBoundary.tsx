import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallbackTitle?: string;
}

interface State {
  error: Error | null;
}

/**
 * React Error Boundary — fängt Render-Errors ab und zeigt sie als UI
 * statt den gesamten Baum zu demounten. Zeigt Error-Message, Stack
 * und einen Reload-Button (Seite neu laden via `window.location.reload()`).
 *
 * Verwendung:
 *   <ErrorBoundary>
 *     <App />
 *   </ErrorBoundary>
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // eslint-disable-next-line no-console
    console.error("ErrorBoundary caught:", error, info);
  }

  handleReload = (): void => {
    window.location.reload();
  };

  handleDismiss = (): void => {
    this.setState({ error: null });
  };

  render(): ReactNode {
    const { error } = this.state;
    const { children, fallbackTitle } = this.props;

    if (error) {
      return (
        <div className="error-boundary">
          <div className="error-boundary-card">
            <h2>{fallbackTitle ?? "Ein Fehler ist aufgetreten"}</h2>
            <p className="error-boundary-message">
              <strong>{error.name}:</strong> {error.message}
            </p>
            {error.stack && (
              <details className="error-boundary-stack">
                <summary>Stack-Trace</summary>
                <pre>{error.stack}</pre>
              </details>
            )}
            <div className="error-boundary-actions">
              <button type="button" onClick={this.handleReload}>
                Neu laden
              </button>
              <button type="button" onClick={this.handleDismiss}>
                Schließen
              </button>
            </div>
          </div>
        </div>
      );
    }

    return children;
  }
}

export default ErrorBoundary;