// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
  },
  compilerOptions: {
    runes: true,
    warningFilter: (warning) => {
        // We can throw here to make warnings fatal, or log and throw.
        // Actually, svelte 5 compilerOptions doesn't use `onwarn` directly inside compilerOptions in svelte.config.js usually, wait!
        // Svelte 5 compiler options has `warningFilter`? No, let's use the root `onwarn` hook.
        return true;
    }
  },
  onwarn: (warning, handler) => {
      // Throw an error on ANY warning so the user catches them at compile time.
      throw new Error(`Svelte Compiler Warning treated as Error: ${warning.message}\nCode: ${warning.code}`);
  }
};

export default config;
