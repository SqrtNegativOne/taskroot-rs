/** Runs `setup` inside a Svelte `$effect.root` for `.browser.test.ts` suites. */
export function mountEffectRoot(setup: () => void): () => void {
    return $effect.root(setup);
}
