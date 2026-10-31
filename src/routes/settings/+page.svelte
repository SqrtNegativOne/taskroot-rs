<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import { describeAppError } from '$lib/errors';
    import { Routes } from '$lib/routes';
    import type { Account } from '$lib/bindings/Account.generated';
    import { STORE_UPDATED } from '$lib/ipcEvents';
    import SettingRow from './SettingRow.svelte';
    import type { AppSettings, SettingsSchema, SettingValue } from './schema';

    const ACCOUNT_TAB_ID = 'account';

    let schema = $state<SettingsSchema | null>(null);
    let settings = $state<AppSettings | null>(null);
    let account = $state<Account | null>(null);
    let activeTabId = $state(ACCOUNT_TAB_ID);
    let saveError = $state<string | null>(null);
    let accountError = $state<string | null>(null);
    let isRemovingAccount = $state(false);

    onMount(() => {
        void initialize();
    });

    async function initialize(): Promise<void> {
        await Promise.all([loadSchema(), loadSettings(), loadAccount()]);
    }

    async function loadSchema(): Promise<void> {
        const result = await safeInvoke<SettingsSchema>('get_settings_schema');
        result.match(
            (loaded) => {
                schema = loaded;
            },
            (error) => console.error('Failed to load schema:', error)
        );
    }

    async function loadSettings(): Promise<void> {
        const result = await safeInvoke<AppSettings>('get_settings');
        result.match(
            (loaded) => (settings = loaded),
            (error) => console.error('Failed to load settings:', error)
        );
    }

    async function loadAccount(): Promise<void> {
        const result = await safeInvoke<Account | null>('get_account');
        result.match(
            (loaded) => (account = loaded),
            (error) => (accountError = describeAppError(error))
        );
    }

    async function removeAccount(): Promise<void> {
        isRemovingAccount = true;
        accountError = null;
        const result = await safeInvoke('disconnect_account');
        isRemovingAccount = false;

        if (result.isErr()) {
            accountError = `Could not remove the account: ${describeAppError(result.error)}`;
            return;
        }
        account = null;
        await goto(resolve(Routes.CONNECT));
    }

    async function handleSettingChange(id: string, value: SettingValue): Promise<void> {
        saveError = null;
        const result = await safeInvoke('update_setting', { key: id, value });
        if (result.isErr()) {
            console.error(`Failed to update setting ${id}:`, result.error);
            saveError = `Could not save "${id}". The change was discarded.`;
            return;
        }
        if (id === 'mini_tracker_enabled' && typeof value === 'boolean') {
            const visibility = await safeInvoke('set_minitracker_visible', { visible: value });
            if (visibility.isErr()) {
                saveError = 'Could not update the mini tracker window.';
            }
        }
        const { emit } = await import('@tauri-apps/api/event');
        void emit(STORE_UPDATED);
        await loadSettings();
    }
</script>

<div class="settings-container">
    {#if schema}
        <div class="sidebar">
            <nav>
                <button
                    class:active={activeTabId === ACCOUNT_TAB_ID}
                    onclick={() => (activeTabId = ACCOUNT_TAB_ID)}
                >
                    Account
                </button>
                {#each schema.tabs as tab (tab.id)}
                    <button
                        class:active={activeTabId === tab.id}
                        onclick={() => (activeTabId = tab.id)}
                    >
                        {tab.label}
                    </button>
                {/each}
            </nav>
        </div>

        <div class="content">
            {#if saveError}
                <div class="save-error" role="alert">{saveError}</div>
            {/if}

            {#if activeTabId === ACCOUNT_TAB_ID}
                <div class="tab-content">
                    <h1>Account</h1>

                    {#if accountError}
                        <div class="save-error" role="alert">{accountError}</div>
                    {/if}

                    {#if account}
                        <section class="account-card">
                            <div class="account-copy">
                                <h2>CalDAV account</h2>
                                <p class="account-server">{account.username} · {account.serverUrl}</p>
                                <p class="account-detail">
                                    {account.calendars.length}
                                    {account.calendars.length === 1 ? 'calendar' : 'calendars'} ·
                                    home {account.homeHref}
                                </p>
                            </div>
                            <button
                                class="remove-account"
                                onclick={removeAccount}
                                disabled={isRemovingAccount}
                            >
                                {isRemovingAccount ? 'Removing…' : 'Remove account'}
                            </button>
                        </section>
                    {:else}
                        <p class="account-empty">No account connected.</p>
                    {/if}
                </div>
            {:else}
                {#each schema.tabs as tab (tab.id)}
                    {#if activeTabId === tab.id}
                        <div class="tab-content">
                            <h1>{tab.label}</h1>
                            {#each tab.sections as section (section.name)}
                                <section class="settings-section">
                                    <h2>{section.name}</h2>
                                    <div class="settings-list">
                                        {#each section.settings as item (item.id)}
                                            <SettingRow
                                                {item}
                                                value={settings?.[item.id]}
                                                onchange={(value: SettingValue) => handleSettingChange(item.id, value)}
                                            />
                                        {/each}
                                    </div>
                                </section>
                            {/each}
                        </div>
                    {/if}
                {/each}
            {/if}
        </div>
    {:else}
        <div class="loading">Loading settings...</div>
    {/if}
</div>

<style>
    .settings-container {
        display: flex;
        height: 100%;
        background-color: var(--bg);
        color: var(--fg);
        overflow: hidden;
    }

    .save-error {
        margin-bottom: 20px;
        padding: 10px 14px;
        border: 1px solid var(--tag-red);
        border-radius: 4px;
        color: var(--tag-red);
        font-size: 13px;
    }

    .account-card {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 20px;
        margin-bottom: 30px;
        padding: 14px 18px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 6px;
    }

    .account-copy h2 {
        margin: 0 0 4px;
        font-size: 13px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--fg-dim);
    }

    .account-server {
        margin: 0;
        font-size: 14px;
    }

    .account-detail {
        margin: 4px 0 0;
        font-size: 12px;
        color: var(--fg-dim);
    }

    .account-empty {
        color: var(--fg-dim);
        font-size: 14px;
    }

    .remove-account {
        flex-shrink: 0;
        padding: 7px 12px;
        font-size: 13px;
        color: var(--tag-red);
        background: transparent;
        border: 1px solid var(--tag-red);
        border-radius: 4px;
        cursor: pointer;
    }

    .remove-account:disabled {
        opacity: 0.6;
        cursor: default;
    }

    .sidebar {
        width: 200px;
        border-right: 1px solid var(--border);
        padding: 20px 0;
        background-color: var(--bg-highlight);
    }

    nav {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    nav button {
        text-align: left;
        padding: 10px 20px;
        background: none;
        border: none;
        cursor: pointer;
        font-size: 14px;
        color: var(--fg-muted);
        border-radius: 0;
    }

    nav button:hover {
        background-color: var(--bg-surface-hover);
    }

    nav button.active {
        background-color: var(--bg-active);
        font-weight: bold;
        color: var(--fg);
    }

    .content {
        flex: 1;
        padding: 30px 40px;
        overflow-y: auto;
    }

    .tab-content h1 {
        margin-top: 0;
        margin-bottom: 30px;
        font-size: 24px;
    }

    .settings-section {
        margin-bottom: 40px;
    }

    .settings-section h2 {
        font-size: 16px;
        color: var(--fg-dim);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 15px;
        border-bottom: 1px solid var(--border);
        padding-bottom: 5px;
    }

    .settings-list {
        display: flex;
        flex-direction: column;
        gap: 20px;
    }

    .loading {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        width: 100%;
        color: var(--fg-dim);
    }
</style>
