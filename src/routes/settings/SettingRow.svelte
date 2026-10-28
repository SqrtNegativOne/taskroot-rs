<script lang="ts">
    import type { SettingsSchemaItem, SettingValue } from './schema';
    import ToggleSwitch from '../../components/inputs/ToggleSwitch.svelte';
    import KeybindingInput from '../../components/inputs/KeybindingInput.svelte';

    interface Props {
        item: SettingsSchemaItem;
        value: SettingValue | undefined;
        onchange: (value: SettingValue) => void;
    }

    let { item, value, onchange }: Props = $props();

    function selectValue(): SettingValue | undefined {
        return value ?? item.defaultValue;
    }

    // A <select> value is always a string; map it back to the option's typed
    // value so numeric settings (e.g. default_task_duration) are not stored as
    // strings the backend then rejects.
    function selectedValue(raw: string): SettingValue {
        const picked = item.options?.find((option) => String(option.value) === raw);
        return picked ? picked.value : raw;
    }
</script>

<div class="setting-item">
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
                onchange={(e) => onchange(selectedValue(e.currentTarget.value))}
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
        border: 1px solid var(--border);
        border-radius: 4px;
        background-color: var(--bg-surface);
        color: var(--fg);
        font-size: 14px;
        width: 100%;
        max-width: 200px;
    }

    :global(input[type='checkbox']) {
        width: 18px;
        height: 18px;
    }
</style>
