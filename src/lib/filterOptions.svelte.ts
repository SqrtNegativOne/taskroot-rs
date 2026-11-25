import { useAutoQuery } from './safeInvoke.svelte';

/** Tag values offered by the filter and sort editors. */
export function useTagOptions(): () => { label: string; value: string }[] {
    const query = useAutoQuery<string[]>('list_tags', () => ({}));
    return () => (query.data ?? []).map(name => ({ label: name, value: name }));
}
