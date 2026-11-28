<script lang="ts">
    import { untrack } from 'svelte';
    import SelectInput from '../inputs/SelectInput.svelte';
    import { addMonths, ymd } from '../../lib/time';
    import {
        WEEKDAY_ORDER,
        buildCustomRule,
        describeRule,
        type CustomFrequency,
        type CustomRecurrence,
        type RecurrenceEndKind,
        type RecurrenceWeekday,
    } from './recurrence';

    interface Props {
        initial: CustomRecurrence;
        replacesAdvanced?: boolean;
        disabled?: boolean;
        onapply: (rule: string) => void;
        oncancel: () => void;
    }

    let {
        initial,
        replacesAdvanced = false,
        disabled = false,
        onapply,
        oncancel,
    }: Props = $props();

    const WEEKDAY_META: readonly { code: RecurrenceWeekday; short: string; label: string }[] = [
        { code: 'MO', short: 'M', label: 'Monday' },
        { code: 'TU', short: 'T', label: 'Tuesday' },
        { code: 'WE', short: 'W', label: 'Wednesday' },
        { code: 'TH', short: 'T', label: 'Thursday' },
        { code: 'FR', short: 'F', label: 'Friday' },
        { code: 'SA', short: 'S', label: 'Saturday' },
        { code: 'SU', short: 'S', label: 'Sunday' },
    ];

    const END_OPTIONS: { value: RecurrenceEndKind; label: string }[] = [
        { value: 'never', label: 'Never' },
        { value: 'count', label: 'After' },
        { value: 'until', label: 'On date' },
    ];

    let freq = $state<CustomFrequency>(untrack(() => initial.freq));
    let interval = $state<number>(untrack(() => initial.interval));
    let byweekday = $state<RecurrenceWeekday[]>(untrack(() => [...initial.byweekday]));
    let endKind = $state<RecurrenceEndKind>(untrack(() => initial.endKind));
    let count = $state<number>(untrack(() => initial.count));
    let until = $state<string>(untrack(() => initial.until));

    let frequencyOptions = $derived([
        { value: 'daily', label: interval === 1 ? 'day' : 'days' },
        { value: 'weekly', label: interval === 1 ? 'week' : 'weeks' },
        { value: 'monthly', label: interval === 1 ? 'month' : 'months' },
        { value: 'yearly', label: interval === 1 ? 'year' : 'years' },
    ]);

    let previewRule = $derived(
        buildCustomRule({ freq, interval, byweekday, endKind, count, until }),
    );
    let previewText = $derived(describeRule(previewRule));
    let canApply = $derived(endKind !== 'until' || /^\d{4}-\d{2}-\d{2}$/.test(until));

    function isCustomFrequency(value: string): value is CustomFrequency {
        return value === 'daily' || value === 'weekly' || value === 'monthly' || value === 'yearly';
    }

    function isEndKind(value: string): value is RecurrenceEndKind {
        return value === 'never' || value === 'count' || value === 'until';
    }

    function handleFrequency(next: string): void {
        if (!isCustomFrequency(next)) return;
        freq = next;
    }

    function handleInterval(event: Event & { currentTarget: EventTarget & HTMLInputElement }): void {
        const parsed = Number.parseInt(event.currentTarget.value, 10);
        interval = Number.isFinite(parsed) && parsed >= 1 ? Math.min(999, parsed) : 1;
    }

    function toggleWeekday(code: RecurrenceWeekday): void {
        const next = byweekday.includes(code)
            ? byweekday.filter((day) => day !== code)
            : WEEKDAY_ORDER.filter((day) => day === code || byweekday.includes(day));
        byweekday = next;
    }

    function handleEndKind(next: string): void {
        if (!isEndKind(next)) return;
        endKind = next;
        if (next === 'until' && !until) {
            until = ymd(addMonths(new Date(), 1));
        }
    }

    function handleCount(event: Event & { currentTarget: EventTarget & HTMLInputElement }): void {
        const parsed = Number.parseInt(event.currentTarget.value, 10);
        count = Number.isFinite(parsed) && parsed >= 1 ? Math.min(999, parsed) : 1;
    }

    function handleUntil(event: Event & { currentTarget: EventTarget & HTMLInputElement }): void {
        until = event.currentTarget.value;
    }

    function handleApply(): void {
        if (!canApply) return;
        onapply(previewRule);
    }
</script>

<div class="inspector-recurrence-custom" role="region" aria-label="Custom recurrence">
    <div class="inspector-recurrence-row">
        <span class="inspector-recurrence-label">Every</span>
        <input
            type="number"
            min="1"
            max="999"
            class="inspector-recurrence-number"
            aria-label="Repeat interval"
            value={interval}
            {disabled}
            oninput={handleInterval}
        />
        <SelectInput
            class="inspector-recurrence-unit"
            value={freq}
            onchange={handleFrequency}
            options={frequencyOptions}
            {disabled}
            ariaLabel="Repeat frequency"
        />
    </div>

    {#if freq === 'weekly'}
        <div class="inspector-recurrence-days" role="group" aria-label="Repeat on days">
            {#each WEEKDAY_META as day (day.code)}
                <button
                    type="button"
                    class="inspector-recurrence-day"
                    class:is-active={byweekday.includes(day.code)}
                    aria-label={day.label}
                    aria-pressed={byweekday.includes(day.code)}
                    title={day.label}
                    {disabled}
                    onclick={() => toggleWeekday(day.code)}
                >
                    {day.short}
                </button>
            {/each}
        </div>
    {/if}

    <div class="inspector-recurrence-row">
        <span class="inspector-recurrence-label">Ends</span>
        <SelectInput
            class="inspector-recurrence-end"
            value={endKind}
            onchange={handleEndKind}
            options={END_OPTIONS}
            {disabled}
            ariaLabel="Recurrence end"
        />
        {#if endKind === 'count'}
            <input
                type="number"
                min="1"
                max="999"
                class="inspector-recurrence-number"
                aria-label="Repeat count"
                value={count}
                {disabled}
                oninput={handleCount}
            />
            <span class="inspector-recurrence-label">{count === 1 ? 'time' : 'times'}</span>
        {:else if endKind === 'until'}
            <input
                type="date"
                class="inspector-recurrence-date"
                aria-label="Repeat until date"
                value={until}
                {disabled}
                oninput={handleUntil}
            />
        {/if}
    </div>

    <p class="inspector-recurrence-preview" aria-live="polite">{previewText}</p>

    {#if replacesAdvanced}
        <p class="inspector-recurrence-note">Applying will replace the advanced rule.</p>
    {/if}

    <div class="inspector-recurrence-footer">
        <button
            type="button"
            class="inspector-recurrence-btn"
            {disabled}
            onclick={oncancel}
        >
            Cancel
        </button>
        <button
            type="button"
            class="inspector-recurrence-btn is-primary"
            disabled={disabled || !canApply}
            onclick={handleApply}
        >
            Apply
        </button>
    </div>
</div>
