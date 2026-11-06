<script lang="ts">
    import SelectInput from '../inputs/SelectInput.svelte';
    import {
        RECURRENCE_OPTIONS,
        presetForRule,
        ruleForPreset,
        type RecurrencePreset,
    } from './recurrence';

    interface Props {
        value?: string;
        onchange: (value: string | undefined) => void;
        disabled?: boolean;
    }

    let { value, onchange, disabled = false }: Props = $props();

    let advanced = $state(false);
    let preset = $derived(presetForRule(value));

    function isRecurrencePreset(candidate: string): candidate is RecurrencePreset {
        return RECURRENCE_OPTIONS.some((option) => option.value === candidate);
    }

    function handlePreset(next: string): void {
        if (!isRecurrencePreset(next)) return;
        if (next === 'custom') {
            advanced = true;
            return;
        }
        onchange(ruleForPreset(next));
    }

    function handleRawRule(event: Event & { currentTarget: EventTarget & HTMLInputElement }): void {
        onchange(event.currentTarget.value.trim() || undefined);
    }
</script>

<div class="inspector-recurrence">
    <SelectInput
        value={preset}
        onchange={handlePreset}
        options={[...RECURRENCE_OPTIONS]}
        variant="bare"
        disabled={disabled}
        ariaLabel="Repeat"
    />
    <button
        type="button"
        class="inspector-advanced-toggle"
        aria-expanded={advanced}
        disabled={disabled}
        onclick={() => { advanced = !advanced; }}
    >
        <span class="material-symbols-outlined" aria-hidden="true">
            {advanced ? 'expand_less' : 'expand_more'}
        </span>
        Advanced
    </button>
    {#if advanced}
        <input
            type="text"
            class="inspector-raw-rule"
            value={value ?? ''}
            placeholder="RRULE:FREQ=WEEKLY"
            aria-label="Raw recurrence rule"
            {disabled}
            onchange={handleRawRule}
            spellcheck="false"
        />
    {/if}
</div>
