import { useTags } from './sharedQueries.svelte';

/** Tag values offered by the filter and sort editors. */
export function useTagOptions(): () => { label: string; value: string }[] {
    const query = useTags();
    return () => (query.data ?? []).map(name => ({ label: name, value: name }));
}
