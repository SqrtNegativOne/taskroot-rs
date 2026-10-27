import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// This is a single-user, pointer-driven desktop app. The accessibility lints
// below demand a keyboard/markup model that would force a UI rewrite without a
// user who needs it, so the policy lives here instead of on individual nodes.
const IGNORED_A11Y_WARNINGS = new Set([
  "a11y_autofocus",
  "a11y_click_events_have_key_events",
  "a11y_no_static_element_interactions",
]);

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
    warningFilter: (warning) => !IGNORED_A11Y_WARNINGS.has(warning.code),
  },
};

export default config;
