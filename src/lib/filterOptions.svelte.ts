import { commands, useAutoQuery } from './safeInvoke.svelte';

/** Tag values offered by the filter and sort editors. */
export function useTagOptions(): () => { label: string; value: string }[] {
    const query = useAutoQuery(commands.listTags);
    return () => (query.data ?? []).map(name => ({ label: name, value: name }));
}
