/** Vitest setup: jest-dom matchers and a clean DOM between tests. */
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

import { clearStoredWidths } from "./app/layout.tsx";

afterEach(() => {
  cleanup();
  clearStoredWidths();
  globalThis.localStorage?.clear();
  globalThis.location.hash = "";
});
