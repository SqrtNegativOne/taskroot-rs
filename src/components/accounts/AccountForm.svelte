<script lang="ts">
    import { untrack } from 'svelte';
    import type { AccountSummary } from '../../lib/domain';
    import { commands, describeAppError, safeInvoke } from '../../lib/safeInvoke.svelte';

    interface Props {
        server?: string;
        user?: string;
        onConnected: () => void | Promise<void>;
        onCancel?: () => void;
        onBusyChange?: (busy: boolean) => void;
    }
    let { server = 'http://127.0.0.1:5232', user = '', onConnected, onCancel, onBusyChange }: Props = $props();
    // The parent keys this form by the profile being reconnected.
    let serverUrl = $state(untrack(() => server));
    let username = $state(untrack(() => user));
    let password = $state('');
    let preview = $state<AccountSummary | null>(null);
    let error = $state<string | null>(null);
    let busy = $state<'test' | 'connect' | null>(null);

    async function discover(connect: boolean): Promise<void> {
        if (busy) return;
        error = null;
        preview = null;
        busy = connect ? 'connect' : 'test';
        onBusyChange?.(true);
        const result = connect
            ? await safeInvoke(commands.connectAccount(serverUrl, username, password))
            : await safeInvoke(commands.testAccount(serverUrl, username, password));
        busy = null;
        onBusyChange?.(false);
        if (result.isErr()) {
            error = describeAppError(result.error);
            return;
        }
        preview = result.value;
        if (connect) {
            password = '';
            await onConnected();
        }
    }
</script>

<form onsubmit={(event) => { event.preventDefault(); void discover(true); }}>
    <p>The password is saved in the OS keychain, never in a config file. Connecting activates this account.</p>
    <label for="account-server">Server URL</label>
    <input id="account-server" type="url" bind:value={serverUrl} autocomplete="url" disabled={busy !== null} required />
    <label for="account-user">Username</label>
    <input id="account-user" bind:value={username} autocomplete="username" disabled={busy !== null} required />
    <label for="account-password">Password</label>
    <input id="account-password" type="password" bind:value={password} autocomplete="current-password" disabled={busy !== null} required />
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="actions">
        {#if onCancel}<button type="button" onclick={onCancel} disabled={busy !== null}>Cancel</button>{/if}
        <button type="button" onclick={() => void discover(false)} disabled={busy !== null}>
            {busy === 'test' ? 'Testing…' : 'Test connection'}
        </button>
        <button class="primary" type="submit" disabled={busy !== null}>
            {busy === 'connect' ? 'Connecting…' : 'Connect'}
        </button>
    </div>
    {#if preview}
        <section aria-live="polite">
            <h3>Found {preview.calendars.length} {preview.calendars.length === 1 ? 'list' : 'lists'}</h3>
            <ul>
                {#each preview.calendars as calendar (calendar.href)}
                    <li>{calendar.displayName || calendar.href} — {calendar.component === 'VEVENT' ? 'Events' : 'Tasks'}{calendar.writable ? '' : ' (read-only)'}</li>
                {/each}
            </ul>
        </section>
    {/if}
</form>

<style>
    form { display: flex; flex-direction: column; gap: 10px; }
    p { margin: 0 0 6px; color: var(--fg-muted); font-size: 13px; line-height: 1.5; }
    label { color: var(--fg-dim); font-size: 13px; }
    input { padding: 9px 11px; font: inherit; color: var(--fg); background: var(--bg-highlight); border: 1px solid var(--border); border-radius: 4px; }
    .actions { display: flex; justify-content: flex-end; flex-wrap: wrap; gap: 10px; margin-top: 10px; }
    button { padding: 8px 12px; color: var(--fg); background: transparent; border: 1px solid var(--border-strong); border-radius: 4px; cursor: pointer; }
    button.primary { color: var(--bg); background: var(--accent); border-color: transparent; }
    button:disabled { opacity: 0.6; cursor: default; }
    .error { padding: 8px 10px; color: var(--tag-red); border: 1px solid var(--tag-red); border-radius: 4px; }
    h3 { font-size: 14px; }
    li { margin: 6px 0; font-size: 13px; }
</style>
