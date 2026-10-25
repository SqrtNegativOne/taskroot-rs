<script lang="ts">
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import { describeAppError } from '$lib/errors';
    import { Routes } from '$lib/routes';
    import type { AccountSummary } from '$lib/bindings/AccountSummary.generated';

    let serverUrl = $state('http://127.0.0.1:5232');
    let username = $state('');
    let password = $state('');
    let preview = $state<AccountSummary | null>(null);
    let error = $state<string | null>(null);
    let busy = $state<'test' | 'connect' | null>(null);

    const isBusy = $derived(busy !== null);

    async function testConnection(): Promise<void> {
        await discover(false);
    }

    async function submit(event: SubmitEvent): Promise<void> {
        event.preventDefault();
        await discover(true);
    }

    async function discover(andConnect: boolean): Promise<void> {
        if (isBusy) return;

        error = null;
        if (!andConnect) preview = null;
        busy = andConnect ? 'connect' : 'test';

        const args = { serverUrl, username, password };
        const result = andConnect
            ? await safeInvoke<AccountSummary>('connect_account', args)
            : await safeInvoke<AccountSummary>('test_account', args);

        busy = null;
        if (result.isErr()) {
            error = describeAppError(result.error);
            return;
        }

        preview = result.value;
        if (andConnect) await goto(resolve(Routes.HOME));
    }
</script>

<div class="connect-shell">
    <form class="connect-card" onsubmit={submit}>
        <header>
            <h1>Connect CalDAV</h1>
            <p>Taskroot stores your calendars on your own CalDAV server. The password goes in the OS keychain, never in a config file.</p>
        </header>

        <label for="server-url">Server URL</label>
        <input
            id="server-url"
            type="url"
            bind:value={serverUrl}
            placeholder="http://127.0.0.1:5232"
            autocomplete="url"
            disabled={isBusy}
            required
        />

        <label for="username">Username</label>
        <input
            id="username"
            type="text"
            bind:value={username}
            autocomplete="username"
            disabled={isBusy}
            required
        />

        <label for="password">Password</label>
        <input
            id="password"
            type="password"
            bind:value={password}
            autocomplete="current-password"
            disabled={isBusy}
            required
        />

        {#if error}
            <p class="connect-error" role="alert">{error}</p>
        {/if}

        <div class="actions">
            <button class="secondary" type="button" onclick={testConnection} disabled={isBusy}>
                {busy === 'test' ? 'Testing…' : 'Test connection'}
            </button>
            <button type="submit" disabled={isBusy}>
                {busy === 'connect' ? 'Connecting…' : 'Connect'}
            </button>
        </div>

        {#if preview}
            <section class="discovered" aria-live="polite">
                <h2>
                    Found {preview.calendars.length}
                    {preview.calendars.length === 1 ? 'calendar' : 'calendars'} on {preview.serverUrl}
                </h2>
                {#if preview.calendars.length === 0}
                    <p class="empty">No calendar collections were advertised by this server.</p>
                {:else}
                    <ul>
                        {#each preview.calendars as calendar (calendar.href)}
                            <li>
                                <span
                                    class="swatch"
                                    style:background={calendar.color ?? 'var(--border-strong)'}
                                ></span>
                                <span class="name">{calendar.displayName || calendar.href}</span>
                                <span class="kind">
                                    {calendar.component === 'VEVENT' ? 'Events' : 'Tasks'}
                                </span>
                                {#if !calendar.writable}
                                    <span class="read-only">read-only</span>
                                {/if}
                            </li>
                        {/each}
                    </ul>
                {/if}
            </section>
        {/if}
    </form>
</div>

<style>
    .connect-shell {
        display: flex;
        align-items: center;
        justify-content: center;
        min-height: 100%;
        padding: 40px 20px;
        background: var(--bg);
        color: var(--fg);
    }

    .connect-card {
        display: flex;
        flex-direction: column;
        gap: 10px;
        width: min(460px, 100%);
        padding: 28px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 8px;
        box-shadow: var(--shadow-btn);
    }

    header {
        margin-bottom: 10px;
    }

    h1 {
        margin: 0 0 8px;
        font-size: 1.4em;
    }

    header p {
        margin: 0;
        font-size: 0.85em;
        line-height: 1.5;
        color: var(--fg-muted);
    }

    label {
        margin-top: 6px;
        font-size: 0.75em;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--fg-dim);
    }

    input {
        padding: 9px 11px;
        font: inherit;
        color: var(--fg);
        background: var(--bg-highlight);
        border: 1px solid var(--border);
        border-radius: 4px;
        outline: none;
    }

    input:focus {
        border-color: var(--accent-dim);
    }

    input:disabled {
        opacity: 0.6;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
        margin-top: 14px;
    }

    button {
        padding: 8px 16px;
        font-size: 0.9em;
        color: var(--bg);
        background: var(--accent);
        border: 1px solid transparent;
        border-radius: 4px;
        cursor: pointer;
    }

    button.secondary {
        color: var(--fg);
        background: transparent;
        border-color: var(--border-strong);
    }

    button:disabled {
        opacity: 0.6;
        cursor: default;
    }

    .connect-error {
        margin: 6px 0 0;
        padding: 8px 10px;
        font-size: 0.85em;
        color: var(--tag-red);
        border: 1px solid var(--tag-red);
        border-radius: 4px;
    }

    .discovered {
        margin-top: 18px;
        padding-top: 14px;
        border-top: 1px solid var(--border);
    }

    .discovered h2 {
        margin: 0 0 10px;
        font-size: 0.8em;
        font-weight: 600;
        color: var(--fg-muted);
    }

    ul {
        display: flex;
        flex-direction: column;
        gap: 6px;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    li {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 0.85em;
    }

    .swatch {
        width: 10px;
        height: 10px;
        border-radius: 2px;
        flex-shrink: 0;
    }

    .name {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .kind {
        color: var(--fg-dim);
        font-size: 0.9em;
    }

    .read-only {
        color: var(--tag-yellow);
        font-size: 0.85em;
    }

    .empty {
        margin: 0;
        font-size: 0.85em;
        color: var(--fg-dim);
    }
</style>
