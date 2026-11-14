/**
 * Catches render errors and offers a report the user can paste into an issue.
 *
 * The report strips absolute paths and anything that looks like a token, because a stack
 * trace from a packaged app contains the user's home directory and the endpoint file's
 * contents can end up in a message.
 */
import { Component, type ErrorInfo, type ReactNode } from "react";

/** Props for {@link ErrorBoundary}. */
export interface ErrorBoundaryProps {
  /** The subtree to guard. */
  readonly children: ReactNode;
  /** Called with the report when an error is caught, for logging. */
  readonly onError?: (report: string) => void;
}

/** State for {@link ErrorBoundary}. */
export interface ErrorBoundaryState {
  /** The report to show, or `null` while nothing has failed. */
  readonly report: string | null;
}

/**
 * Remove absolute paths and token-shaped strings from a report.
 *
 * Windows paths (`C:\Users\name\...`), POSIX home paths (`/home/name/...`) and bare
 * `token=...` pairs are replaced, since none of them help diagnose a render error.
 */
export function stripSensitive(text: string): string {
  return text
    .replace(/[A-Za-z]:\\[^\s"':]+/g, "<path>")
    .replace(/\/(?:home|Users)\/[^\s"':]+/g, "<path>")
    .replace(/\b(token|secret|password|api[_-]?key)\s*[=:]\s*\S+/gi, "$1=<redacted>");
}

/** Build the copyable report for a caught error. */
export function buildReport(error: unknown, info?: ErrorInfo): string {
  const message = error instanceof Error ? error.message : String(error);
  const stack = error instanceof Error && error.stack ? `\n${error.stack}` : "";
  const component = info?.componentStack ? `\ncomponent stack:${info.componentStack}` : "";
  return stripSensitive(`Slate-To-UCS render error\n${message}${stack}${component}`);
}

/** A boundary that renders a report instead of a blank window. */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  override state: ErrorBoundaryState = { report: null };

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    return { report: buildReport(error) };
  }

  override componentDidCatch(error: unknown, info: ErrorInfo): void {
    const report = buildReport(error, info);
    this.setState({ report });
    this.props.onError?.(report);
  }

  override render(): ReactNode {
    const { report } = this.state;
    if (report === null) return this.props.children;
    return (
      <div className="crash" role="alert">
        <h1>Something went wrong</h1>
        <p>The window hit an error it could not recover from. Copy the report below.</p>
        <pre className="crash__report">{report}</pre>
        <button type="button" onClick={() => void navigator.clipboard?.writeText(report)}>
          Copy report
        </button>
        <button type="button" onClick={() => this.setState({ report: null })}>
          Try again
        </button>
      </div>
    );
  }
}
