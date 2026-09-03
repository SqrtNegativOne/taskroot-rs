<script lang="ts">
    import { onMount } from 'svelte';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import SettingRow from './SettingRow.svelte';
    import type { AppSettings, SettingsSchema, SettingValue } from './schema';

    let schema = $state<SettingsSchema | null>(null);
    let settings = $state<AppSettings | null>(null);
    let activeTabId = $state('');
    let saveError = $state<string | null>(null);

    onMount(() => {
        void initialize();
    });

    async function initialize(): Promise<void> {
        await loadSchema();
        await loadSettings();
    }

    async function loadSchema(): Promise<void> {
        const result = await safeInvoke<SettingsSchema>('get_settings_schema');
        result.match(
            (loaded) => {
                schema = loaded;
                activeTabId = loaded.tabs[0]?.id ?? '';
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

    async function handleSettingChange(id: string, value: SettingValue): Promise<void> {
        saveError = null;
        const result = await safeInvoke('update_setting', { key: id, value });
        if (result.isErr()) {
            console.error(`Failed to update setting ${id}:`, result.error);
            saveError = `Could not save "${id}". The change was discarded.`;
            return;
        }
        const { emit } = await import('@tauri-apps/api/event');
        void emit('store-updated');
        await loadSettings();
    }
</script>

<div class="settings-container">
    {#if schema}
        <div class="sidebar">
            <nav>
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
