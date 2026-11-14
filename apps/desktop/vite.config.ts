import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The Tauri shell serves the built assets from `dist`, and the dev server binds a fixed
// port so `tauri.conf.json` can point at it without guessing.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "chrome110",
    sourcemap: true,
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
