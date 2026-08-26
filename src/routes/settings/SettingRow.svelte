<script lang="ts">
    import type { SettingsSchemaItem, SettingValue } from './schema';
    import ToggleSwitch from '../../components/inputs/ToggleSwitch.svelte';
    import KeybindingInput from '../../components/inputs/KeybindingInput.svelte';

    import { safeInvoke } from '$lib/safeInvoke.svelte';

    interface Props {
        item: SettingsSchemaItem;
        value: SettingValue | undefined;
        onchange: (value: SettingValue) => void;
    }

    let { item, value, onchange }: Props = $props();

    function selectValue(): SettingValue | undefined {
        return value ?? item.defaultValue;
    }

    async function handleCustomAction(id: string) {
        if (id === 'logout') {
            if (confirm("Are you sure you want to sign out?")) {
                await safeInvoke('reset_auth');
                window.location.reload();
            }
        } else if (id === 'clear_all_data') {
            if (confirm("Are you sure you want to wipe all local data? This cannot be undone.")) {
                await safeInvoke('wipe_local_data');
                window.location.reload();
            }
        } else {
            console.log(`Custom action clicked: ${id}`);
        }
    }
</script>

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
                value={selectValue()}
                onchange={(e) => onchange(e.currentTarget.value)}
            >
                {#each item.options as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                {/each}
            </select>
        {:else if item.type === 'checkbox'}
            <ToggleSwitch
                checked={Boolean(selectValue())}
                onchange={(checked: boolean) => onchange(checked)}
                ariaLabel={item.label}
            />
        {:else if item.type === 'number'}
            <input
                type="number"
                id={item.id}
                min={item.min}
                max={item.max}
                value={selectValue()}
                oninput={(e) => onchange(Number(e.currentTarget.value))}
            />
        {:else if item.type === 'time'}
            <input
                type="number"
                id={item.id}
                value={selectValue()}
                oninput={(e) => onchange(Number(e.currentTarget.value))}
            />
        {:else if item.type === 'keybinding'}
            <KeybindingInput
                value={selectValue() as string}
                onchange={(val: string) => onchange(val)}
            />
        {:else if item.type === 'custom' || item.type === 'action'}
            <button onclick={() => handleCustomAction(item.id)}>
                {item.label}
            </button>
        {/if}
    </div>
</div>

<style>
    .setting-item {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 20px;
    }

    .setting-item.danger label {
        color: var(--color-danger);
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
        color: var(--fg-dim);
    }

    .setting-control {
        min-width: 150px;
        display: flex;
        justify-content: flex-end;
    }

    :global(input[type='text']),
    :global(input[type='number']),
    select {
        padding: 6px 10px;
        border: 1px solid var(--border-color);
        border-radius: 4px;
        background-color: var(--bg-input);
        color: var(--fg-primary);
        font-size: 14px;
        width: 100%;
        max-width: 200px;
    }

    :global(input[type='checkbox']) {
        width: 18px;
        height: 18px;
    }

    button {
        padding: 6px 12px;
        border: 1px solid var(--border-color);
        border-radius: 4px;
        background-color: var(--bg-button);
        cursor: pointer;
        font-size: 14px;
    }

    button:hover {
        background-color: var(--bg-button-hover);
    }
</style>
