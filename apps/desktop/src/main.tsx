/**
 * Mounts React into `#root`.
 *
 * The endpoint is read from the query string the Tauri shell appends when it loads the
 * webview, so the UI never has to guess a port or read the endpoint file itself.
 */
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App.tsx";
import "./app/theme.css";
import { EngineClient } from "./lib/engine.ts";
import type { Endpoint } from "./lib/engine.ts";

/** Read the endpoint the shell passed in, or `undefined` when it passed none. */
export function endpointFromSearch(search: string): Endpoint | undefined {
  const parameters = new URLSearchParams(search);
  const baseUrl = parameters.get("endpoint");
  const token = parameters.get("token");
  if (baseUrl === null || baseUrl === "" || token === null) return undefined;
  return { baseUrl, token };
}

const container = document.getElementById("root");
if (container === null) {
  throw new Error("#root is missing from index.html");
}

const endpoint = endpointFromSearch(globalThis.location?.search ?? "");
const client = endpoint === undefined ? undefined : new EngineClient({ endpoint });

createRoot(container).render(
  <StrictMode>
    <App {...(client === undefined ? {} : { client })} />
  </StrictMode>,
);
