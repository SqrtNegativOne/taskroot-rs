<script lang="ts">
    import { onMount, untrack } from 'svelte';
    import AccountForm from './AccountForm.svelte';
    import type { AccountProfile } from '../../lib/domain';
    import { commands, describeAppError, safeInvoke, useTauriQuery } from '../../lib/safeInvoke.svelte';
    import { store } from '../../lib/store.svelte';

    interface Props {
        onActivated?: () => void | Promise<void>;
        showAddInitially?: boolean;
    }
    let { onActivated, showAddInitially = false }: Props = $props();
    const accounts = useTauriQuery(commands.getAccounts);
    onMount(() => { void accounts.execute(); });
    let busy = $state<string | null>(null);
    let error = $state<string | null>(null);
    let form = $state<AccountProfile | 'new' | null>(untrack(() => showAddInitially ? 'new' : null));
    const registry = $derived(accounts.data);

    async function switchAccount(id: string): Promise<void> {
        if (busy) return;
        busy = id;
        error = null;
        const result = await store.mutate('Could not switch account', () => safeInvoke(commands.switchAccount(id)));
        busy = null;
        if (result.isErr()) {
            error = describeAppError(result.error);
            return;
        }
        form = null;
        await accounts.execute();
        await onActivated?.();
    }

    async function remove(profile: AccountProfile): Promise<void> {
        if (busy || !confirm(`Forget ${profile.account.username} on ${profile.account.serverUrl}? This removes its saved password, not its server data.`)) return;
        busy = profile.id;
        error = null;
        const result = await store.mutate('Could not remove account', () => safeInvoke(commands.removeAccount(profile.id)));
        busy = null;
        if (result.isErr()) {
            error = describeAppError(result.error);
            return;
        }
        if (form !== 'new' && form?.id === profile.id) form = null;
        await accounts.execute();
    }

    async function disconnect(): Promise<void> {
        if (busy) return;
        busy = 'disconnect';
        error = null;
        const result = await store.mutate('Could not disconnect account', () => safeInvoke(commands.disconnectAccount()));
        busy = null;
        if (result.isErr()) error = describeAppError(result.error);
        else await accounts.execute();
    }

    async function connected(): Promise<void> {
        form = null;
        await accounts.execute();
        await onActivated?.();
    }
</script>

<div class="profiles">
    <p class="hint">One account is active at a time. Switch here without restarting; settings and window state stay unchanged.</p>
    {#if error || accounts.error}
        <p class="error" role="alert">{error ?? describeAppError(accounts.error)}</p>
    {/if}
    {#if registry}
        {#each registry.profiles as profile (profile.id)}
            <section class="profile" aria-label={`${profile.account.username} on ${profile.account.serverUrl}`}>
                <div class="identity">
                    <strong>{profile.account.username}</strong>
                    <span>{profile.account.serverUrl}</span>
                    {#if profile.id === registry.activeId}<span class="active">Active</span>{/if}
                </div>
                <div class="actions">
                    {#if profile.id !== registry.activeId}
                        <button onclick={() => void switchAccount(profile.id)} disabled={busy !== null}>
                            {busy === profile.id ? 'Working…' : 'Switch'}
                        </button>
                    {/if}
                    <button onclick={() => { error = null; form = profile; }} disabled={busy !== null}>Reconnect</button>
                    <button class="danger" onclick={() => void remove(profile)} disabled={busy !== null}>Forget</button>
                </div>
            </section>
        {/each}
        {#if registry.activeId === null}<p class="hint">No account connected. Choose a saved account or add one.</p>{/if}
        <div class="actions">
            <button onclick={() => { error = null; form = 'new'; }} disabled={busy !== null}>Add account</button>
            {#if registry.activeId !== null}
                <button onclick={() => void disconnect()} disabled={busy !== null}>Disconnect</button>
            {/if}
        </div>
    {:else if accounts.isLoading}
        <p class="hint">Loading accounts…</p>
    {/if}
    {#if form !== null}
        <section class="form">
            <h2>{form === 'new' ? 'Add account' : 'Reconnect account'}</h2>
            {#key form === 'new' ? 'new' : form.id}
                <AccountForm server={form === 'new' ? undefined : form.account.serverUrl}
                    user={form === 'new' ? undefined : form.account.username}
                    onConnected={connected} onCancel={() => (form = null)}
                    onBusyChange={(working) => (busy = working ? 'form' : null)} />
            {/key}
        </section>
    {/if}
</div>

<style>
    .profiles { display: flex; flex-direction: column; gap: 14px; }
    .hint { margin: 0; color: var(--fg-dim); font-size: 13px; line-height: 1.5; }
    .profile { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px; padding: 14px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 6px; }
    .identity { display: flex; flex-wrap: wrap; align-items: baseline; gap: 8px; font-size: 14px; }
    .identity > span { color: var(--fg-dim); overflow-wrap: anywhere; }
    .identity .active { color: var(--accent); font-size: 12px; }
    .actions { display: flex; flex-wrap: wrap; gap: 8px; }
    button { padding: 7px 12px; color: var(--fg); background: transparent; border: 1px solid var(--border-strong); border-radius: 4px; cursor: pointer; }
    button.danger { color: var(--tag-red); }
    button:disabled { opacity: 0.6; cursor: default; }
    .error { margin: 0; padding: 10px; border: 1px solid var(--tag-red); color: var(--tag-red); border-radius: 4px; }
    .form { padding: 18px; border: 1px solid var(--border); border-radius: 6px; }
    h2 { margin: 0 0 14px; font-size: 16px; }
</style>
