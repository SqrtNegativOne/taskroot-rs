import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import eslintPluginSvelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import oxlint from 'eslint-plugin-oxlint';
import sonarjs from 'eslint-plugin-sonarjs';

export default tseslint.config(
  {
    ignores: ['.svelte-kit/', 'dist/', 'src-tauri/', 'node_modules/', '*.config.js', '**/*.cjs', 'script.js', 'test_useNow.svelte.ts', 'build/', 'legacy-src/', 'coverage/'],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  ...eslintPluginSvelte.configs['flat/recommended'],
  sonarjs.configs.recommended,
  oxlint.configs['flat/recommended'],
  {
    languageOptions: {
      parserOptions: {
        extraFileExtensions: ['.svelte'],
      },
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.svelte'],
      },
    },
    rules: {
    },
  },
  {
    rules: {
      'no-undef': 'off',
      '@typescript-eslint/explicit-function-return-type': 'off',
      'svelte/valid-compile': 'error',
      'svelte/prefer-svelte-reactivity': 'off',
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/no-unnecessary-condition': 'off',
      'sonarjs/void-use': 'off',
      'sonarjs/todo-tag': 'off',
      'sonarjs/no-nested-conditional': 'off',
      'sonarjs/pseudo-random': 'off',
      'sonarjs/no-identical-functions': 'off',
      'sonarjs/no-use-of-empty-return-value': 'off',
      'sonarjs/redundant-type-aliases': 'off',
    }
  },
  {
    files: ['**/*.generated.ts', '**/globals.d.ts'],
    rules: {
      '@typescript-eslint/array-type': 'off',
      '@typescript-eslint/consistent-type-definitions': 'off',
      'sonarjs/redundant-type-aliases': 'off',
    },
  }
);
