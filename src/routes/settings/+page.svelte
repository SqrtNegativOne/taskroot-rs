<script lang="ts">
    import { onMount } from 'svelte';
    import { safeInvoke } from '$lib/safeInvoke.svelte';

    type SettingOption = { value: any; label: string };

    type SettingSchemaItem = {
        id: string;
        label: string;
        description?: string;
        keywords?: string[];
        type: string;
        options?: SettingOption[];
        min?: number;
        max?: number;
        defaultValue?: any;
        danger?: boolean;
    };

    type SettingSection = {
        name: string;
        settings: SettingSchemaItem[];
    };

    type SettingTab = {
        id: string;
        label: string;
        sections: SettingSection[];
    };

    type SettingsSchema = {
        tabs: SettingTab[];
    };

    let schema = $state<SettingsSchema | null>(null);
    let settings = $state<Record<string, any>>({});
    let activeTabId = $state<string>('');

    onMount(async () => {
        const schemaResult = await safeInvoke<SettingsSchema>('get_settings_schema');
        schemaResult.match(
            (v) => {
                schema = v;
                if (v.tabs.length > 0) {
                    activeTabId = v.tabs[0].id;
                }
            },
            (e) => console.error('Failed to load schema:', e)
        );

        const settingsResult = await safeInvoke<Record<string, any>>('get_settings');
        settingsResult.match(
            (v) => settings = v,
            (e) => console.error('Failed to load settings:', e)
        );
    });

    async function handleSettingChange(id: string, value: any) {
        settings[id] = value;
        const result = await safeInvoke('update_setting', { key: id, value });
        result.match(
            () => {}, // Success
            (e) => console.error(`Failed to update setting ${id}:`, e)
        );
    }
</script>

<div class="settings-container">
    {#if schema}
        <div class="sidebar">
            <nav>
                {#each schema.tabs as tab}
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
            {#each schema.tabs as tab}
                {#if activeTabId === tab.id}
                    <div class="tab-content">
                        <h1>{tab.label}</h1>
                        {#each tab.sections as section}
                            <section class="settings-section">
                                <h2>{section.name}</h2>
                                <div class="settings-list">
                                    {#each section.settings as item}
                                        <div class="setting-item" class:danger={item.danger}>
                                            <div class="setting-info">
                                                <label for={item.id}>{item.label}</label>
                                                {#if item.description}
                                                    <p class="description">{item.description}</p>
                                                {/if}
                                            </div>
                                            <div class="setting-control">
                                                {#if item.type === 'select' && item.options}
                                                    <select
                                                        id={item.id}
                                                        value={settings[item.id] ?? item.defaultValue}
                                                        onchange={(e) => handleSettingChange(item.id, (e.target as HTMLSelectElement).value)}
                                                    >
                                                        {#each item.options as option}
                                                            <option value={option.value}>{option.label}</option>
                                                        {/each}
                                                    </select>
                                                {:else if item.type === 'checkbox'}
                                                    <input
                                                        type="checkbox"
                                                        id={item.id}
                                                        checked={settings[item.id] ?? item.defaultValue}
                                                        onchange={(e) => handleSettingChange(item.id, (e.target as HTMLInputElement).checked)}
                                                    />
                                                {:else if item.type === 'number'}
                                                    <input
                                                        type="number"
                                                        id={item.id}
                                                        min={item.min}
                                                        max={item.max}
                                                        value={settings[item.id] ?? item.defaultValue}
                                                        oninput={(e) => handleSettingChange(item.id, Number((e.target as HTMLInputElement).value))}
                                                    />
                                                {:else if item.type === 'time'}
                                                    <!-- Time is often stored in minutes, but let's just use a number input for simplicity unless we have a specific time picker -->
                                                    <input
                                                        type="number"
                                                        id={item.id}
                                                        value={settings[item.id] ?? item.defaultValue}
                                                        oninput={(e) => handleSettingChange(item.id, Number((e.target as HTMLInputElement).value))}
                                                    />
                                                {:else if item.type === 'keybinding'}
                                                    <input
                                                        type="text"
                                                        id={item.id}
                                                        value={settings[item.id] ?? item.defaultValue}
                                                        oninput={(e) => handleSettingChange(item.id, (e.target as HTMLInputElement).value)}
                                                    />
                                                {:else if item.type === 'custom' || item.type === 'action'}
                                                    <button onclick={() => console.log('Custom action clicked:', item.id)}>
                                                        {item.label}
                                                    </button>
                                                {/if}
                                            </div>
                                        </div>
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
        background-color: var(--bg-primary, #ffffff);
        color: var(--fg-primary, #000000);
        overflow: hidden;
    }

    .sidebar {
        width: 200px;
        border-right: 1px solid var(--border-color, #e0e0e0);
        padding: 20px 0;
        background-color: var(--bg-secondary, #f5f5f5);
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
        color: var(--fg-secondary, #444);
        border-radius: 0;
    }

    nav button:hover {
        background-color: var(--bg-hover, #e0e0e0);
    }

    nav button.active {
        background-color: var(--bg-active, #d0d0d0);
        font-weight: bold;
        color: var(--fg-primary, #000);
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
        color: var(--fg-dim, #888);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 15px;
        border-bottom: 1px solid var(--border-color, #e0e0e0);
        padding-bottom: 5px;
    }

    .settings-list {
        display: flex;
        flex-direction: column;
        gap: 20px;
    }

    .setting-item {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 20px;
    }

    .setting-item.danger label {
        color: var(--color-danger, #d32f2f);
    }

    .setting-info {
        flex: 1;
    }

    label {
        font-weight: 500;
        display: block;
        margin-bottom: 4px;
        font-size: 15px;
    }

    .description {
        margin: 0;
        font-size: 13px;
        color: var(--fg-dim, #888);
    }

    .setting-control {
        min-width: 150px;
        display: flex;
        justify-content: flex-end;
    }

    input[type="text"],
    input[type="number"],
    select {
        padding: 6px 10px;
        border: 1px solid var(--border-color, #ccc);
        border-radius: 4px;
        background-color: var(--bg-input, #fff);
        color: var(--fg-primary, #000);
        font-size: 14px;
        width: 100%;
        max-width: 200px;
    }

    input[type="checkbox"] {
        width: 18px;
        height: 18px;
    }

    button {
        padding: 6px 12px;
        border: 1px solid var(--border-color, #ccc);
        border-radius: 4px;
        background-color: var(--bg-button, #f0f0f0);
        cursor: pointer;
        font-size: 14px;
    }

    button:hover {
        background-color: var(--bg-button-hover, #e0e0e0);
    }

    .loading {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        width: 100%;
        color: var(--fg-dim, #888);
    }
</style>
