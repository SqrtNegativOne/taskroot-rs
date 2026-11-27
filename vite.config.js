import { defineConfig } from "vitest/config";
import { sveltekit } from "@sveltejs/kit/vite";
import { playwright } from "@vitest/browser-playwright";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [sveltekit()],
  test: {
    passWithNoTests: true,
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      include: ["src/**"],
      exclude: ["src/lib/bindings/**", "**/*.d.ts"],
    },
    projects: [
      {
        // Pure TypeScript tests: no DOM, no SvelteKit plugin, no setup overhead.
        test: {
          name: "unit",
          environment: "node",
          isolate: false,
          include: ["src/**/*.{test,spec}.{js,ts}"],
          exclude: ["src/**/*.browser.{test,spec}.{js,ts}"],
        },
      },
      {
        // Component, Svelte-rune, and DOM tests running in real headless Chromium.
        extends: true,
        test: {
          name: "browser",
          setupFiles: ["./src/test/setup.ts"],
          include: ["src/**/*.browser.{test,spec}.{js,ts}"],
          browser: {
            enabled: true,
            headless: true,
            screenshotFailures: false,
            provider: playwright(),
            instances: [{ browser: "chromium" }],
          },
        },
      },
    ],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
          timeout: 60000,
        }
      : {
          timeout: 60000,
        },
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
});
