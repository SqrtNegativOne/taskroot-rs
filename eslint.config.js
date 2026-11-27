import tseslint from 'typescript-eslint';
import eslintPluginSvelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import oxlint from 'eslint-plugin-oxlint';

// Oxlint owns JS/TS rules, type-aware rules, complexity, Vitest rules, and
// curated SonarJS rules across all `.ts` and `.svelte` `<script>` blocks.
// ESLint only runs Svelte template rules that Oxlint cannot parse yet.
export default [
  {
    ignores: ['.svelte-kit/', 'dist/', 'src-tauri/', 'node_modules/', 'build/', 'coverage/', 'src/lib/bindings/'],
  },
  ...eslintPluginSvelte.configs['flat/recommended'].map((config) => ({
    ...config,
    files: config.files ?? ['**/*.svelte'],
  })),
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.svelte'],
      },
    },
    rules: {
      // Svelte compiler errors and warnings (with `svelte.config.js` a11y
      // policy) are owned by `svelte-check`.
      'svelte/valid-compile': 'off',
      'svelte/prefer-svelte-reactivity': 'off',
    },
  },
  ...oxlint.buildFromOxlintConfigFile('./.oxlintrc.json'),
];
