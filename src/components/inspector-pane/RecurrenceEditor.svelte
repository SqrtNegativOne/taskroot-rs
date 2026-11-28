<script lang="ts">
    import SelectInput from '../inputs/SelectInput.svelte';
    import RecurrenceCustomForm from './RecurrenceCustomForm.svelte';
    import {
        DEFAULT_CUSTOM_RECURRENCE,
        RECURRENCE_OPTIONS,
        describeRule,
        isValidRule,
        parseCustomRule,
        presetForRule,
        ruleForPreset,
        type CustomRecurrence,
        type RecurrencePreset,
    } from './recurrence';

    interface Props {
        value?: string | null;
        onchange: (value: string | undefined) => void;
        disabled?: boolean;
    }

    let { value, onchange, disabled = false }: Props = $props();

    let customOpen = $state(false);
    let advanced = $state(false);
    let rawDraft = $derived(value ?? '');

    let preset = $derived(presetForRule(value ?? undefined));
    let isCustom = $derived(preset === 'custom');
    let summary = $derived(describeRule(value ?? undefined));
    let parsedCustom = $derived(parseCustomRule(value ?? undefined));
    let replacesAdvanced = $derived(isCustom && parsedCustom === null);
    let initialCustom = $derived<CustomRecurrence>(parsedCustom ?? DEFAULT_CUSTOM_RECURRENCE);

    let selectOptions = $derived(
        RECURRENCE_OPTIONS.map((option) =>
            option.value === 'custom' && isCustom
                ? { value: option.value, label: summary }
                : option,
        ),
    );

    let rawTrimmed = $derived(rawDraft.trim());
    let rawValid = $derived(rawTrimmed === '' || isValidRule(rawTrimmed));
    let rawPreview = $derived(rawTrimmed && rawValid ? describeRule(rawTrimmed) : '');

    function isRecurrencePreset(candidate: string): candidate is RecurrencePreset {
        return RECURRENCE_OPTIONS.some((option) => option.value === candidate);
    }

    function handlePreset(next: string): void {
        if (!isRecurrencePreset(next)) return;
        if (next === 'custom') {
            customOpen = true;
            return;
        }
        customOpen = false;
        onchange(ruleForPreset(next));
    }

    function handleCustomApply(rule: string): void {
        customOpen = false;
        onchange(rule);
    }

    function handleRawInput(event: Event & { currentTarget: EventTarget & HTMLInputElement }): void {
        rawDraft = event.currentTarget.value;
    }

    function handleRawCommit(event: Event & { currentTarget: EventTarget & HTMLInputElement }): void {
        const next = event.currentTarget.value.trim();
        rawDraft = next;
        if (!next) {
            onchange(undefined);
            return;
        }
        if (isValidRule(next)) {
            onchange(next);
        }
    }
</script>

<div class="inspector-recurrence">
    <SelectInput
        value={preset}
        onchange={handlePreset}
        options={selectOptions}
        variant="bare"
        disabled={disabled}
        ariaLabel="Repeat"
    />

    <div class="inspector-recurrence-actions">
        {#if isCustom || customOpen}
            <button
                type="button"
                class="inspector-advanced-toggle"
                aria-expanded={customOpen}
                {disabled}
                onclick={() => { customOpen = !customOpen; }}
            >
                <span class="material-symbols-outlined" aria-hidden="true">tune</span>
                {customOpen ? 'Hide custom' : 'Customize'}
            </button>
        {/if}
        <button
            type="button"
            class="inspector-advanced-toggle"
            aria-expanded={advanced}
            {disabled}
            onclick={() => { advanced = !advanced; }}
        >
            <span class="material-symbols-outlined" aria-hidden="true">
                {advanced ? 'expand_less' : 'expand_more'}
            </span>
            Advanced
        </button>
    </div>

    {#if customOpen}
        <RecurrenceCustomForm
            initial={initialCustom}
            {replacesAdvanced}
            {disabled}
            onapply={handleCustomApply}
            oncancel={() => { customOpen = false; }}
        />
    {/if}

    {#if advanced}
        <input
            type="text"
            class="inspector-raw-rule"
            class:is-invalid={!rawValid}
            value={rawDraft}
            placeholder="RRULE:FREQ=WEEKLY"
            aria-label="Raw recurrence rule"
            aria-invalid={!rawValid}
            {disabled}
            oninput={handleRawInput}
            onchange={handleRawCommit}
            spellcheck="false"
        />
        {#if !rawValid}
            <p class="inspector-recurrence-error" role="alert">
                Enter a valid RFC 5545 rule (e.g. RRULE:FREQ=WEEKLY;INTERVAL=2).
            </p>
        {:else if rawPreview}
            <p class="inspector-recurrence-preview">{rawPreview}</p>
        {/if}
    {/if}
</div>
