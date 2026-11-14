/**
 * A two-route hash router.
 *
 * The app has exactly two screens, so a router library would be more surface than the
 * problem needs. The hash is the source of truth, which keeps the Tauri webview's history
 * behaviour identical to a browser's and makes the route testable without a DOM.
 */
import { useSyncExternalStore } from "react";

/** The screens the shell can show. */
export type Route = "workspace" | "settings";

/** Every route, in navigation order. */
export const ROUTES: readonly Route[] = ["workspace", "settings"];

/** The route a hash points at, defaulting to the workspace. */
export function parseRoute(hash: string): Route {
  const trimmed = hash.replace(/^#\/?/, "").replace(/\/+$/, "");
  return trimmed === "settings" ? "settings" : "workspace";
}

/** The hash that selects a route. */
export function routeHref(route: Route): string {
  return `#/${route}`;
}

/** The label shown in the navigation. */
export function routeLabel(route: Route): string {
  return route === "settings" ? "Settings" : "Workspace";
}

function subscribe(onChange: () => void): () => void {
  globalThis.addEventListener("hashchange", onChange);
  return () => globalThis.removeEventListener("hashchange", onChange);
}

function currentHash(): string {
  return globalThis.location?.hash ?? "";
}

/** The active route, re-rendering when the hash changes. */
export function useRoute(): Route {
  const hash = useSyncExternalStore(subscribe, currentHash, () => "");
  return parseRoute(hash);
}

/** Move to a route by setting the hash. */
export function navigate(route: Route): void {
  globalThis.location.hash = routeHref(route);
  // A webview may deliver `hashchange` on the next task; dispatching it here keeps the
  // navigation synchronous for the caller and for tests.
  globalThis.dispatchEvent(new Event("hashchange"));
}
