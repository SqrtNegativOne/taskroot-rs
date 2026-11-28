<script lang="ts">
    import { untrack } from 'svelte';
    import { DatePicker } from 'bits-ui';
    import type { DateValue } from '@internationalized/date';
    import './picker.css';
    import TimeInput from '../inputs/TimeInput.svelte';
    import {
        combineDateAndTime,
        formatTaskDateTime,
        getFormattedDate,
        getFormattedTime,
        parseYmdToCalendarDate,
        todayCalendarDate,
    } from './format';

    interface Props {
        value?: string | null;
        timezone?: string | null;
        placeholder: string;
        ariaLabel: string;
        timeAriaLabel: string;
        disabled?: boolean;
        onchange: (value: string | null) => void;
    }

    let {
        value = null,
        timezone = null,
        placeholder,
        ariaLabel,
        timeAriaLabel,
        disabled = false,
        onchange,
    }: Props = $props();

    let open = $state(false);
    let selectedDate = $derived(getFormattedDate(value, timezone));
    let selectedTime = $derived(getFormattedTime(value, timezone));
    let calendarPlaceholder = $state<DateValue>(
        untrack(() => parseYmdToCalendarDate(getFormattedDate(value, timezone)) ?? todayCalendarDate()),
    );

    const calendarValue = $derived(parseYmdToCalendarDate(selectedDate));
    const displayText = $derived(
        formatTaskDateTime(combineDateAndTime(selectedDate, selectedTime), placeholder),
    );

    function handleOpenChange(nextOpen: boolean): void {
        open = nextOpen;
        if (nextOpen) {
            calendarPlaceholder = parseYmdToCalendarDate(selectedDate) ?? todayCalendarDate();
            return;
        }
        selectedDate = getFormattedDate(value, timezone);
        selectedTime = getFormattedTime(value, timezone);
    }

    function handleDateSelect(next: DateValue | undefined): void {
        if (!next || disabled) return;
        selectedDate = next.toString();
        onchange(combineDateAndTime(selectedDate, selectedTime));
    }

    function handleTimeChange(nextTime: string): void {
        if (!selectedDate || disabled) return;
        selectedTime = nextTime;
        onchange(combineDateAndTime(selectedDate, selectedTime));
    }

    function handleClear(): void {
        if (disabled) return;
        selectedDate = '';
        selectedTime = '';
        onchange(null);
    }
</script>

<DatePicker.Root
    bind:open
    onOpenChange={handleOpenChange}
    value={calendarValue}
    onValueChange={handleDateSelect}
    bind:placeholder={calendarPlaceholder}
    weekStartsOn={1}
    weekdayFormat="short"
    fixedWeeks={true}
    preventDeselect={true}
    closeOnDateSelect={false}
    calendarLabel={ariaLabel}
    {disabled}
>
    <div class="inspector-picker-row">
        <DatePicker.Trigger
            class={`inspector-picker-trigger ${selectedDate ? '' : 'is-empty'}`.trim()}
            aria-label={ariaLabel}
            {disabled}
        >
            <span class="inspector-picker-label">{displayText}</span>
        </DatePicker.Trigger>
        {#if selectedDate && !disabled}
            <button
                type="button"
                class="inspector-clear"
                aria-label={`Clear ${ariaLabel}`}
                onclick={handleClear}
            >
                <span class="material-symbols-outlined" style="font-size: 14px;">close</span>
            </button>
        {/if}
    </div>

    <DatePicker.Portal>
        <DatePicker.Content
            sideOffset={6}
            align="start"
            class="inspector-date-popover"
            data-inspector-popover={open ? '' : undefined}
        >
            <DatePicker.Calendar>
                {#snippet children({ months, weekdays })}
                    <DatePicker.Header class="inspector-cal-header">
                        <DatePicker.PrevButton class="inspector-cal-nav" aria-label="Previous month">
                            <span class="material-symbols-outlined" aria-hidden="true">chevron_left</span>
                        </DatePicker.PrevButton>
                        <DatePicker.Heading class="inspector-cal-heading" />
                        <DatePicker.NextButton class="inspector-cal-nav" aria-label="Next month">
                            <span class="material-symbols-outlined" aria-hidden="true">chevron_right</span>
                        </DatePicker.NextButton>
                    </DatePicker.Header>

                    {#each months as month (month.value.toString())}
                        <DatePicker.Grid class="inspector-cal-grid">
                            <DatePicker.GridHead>
                                <DatePicker.GridRow>
                                    {#each weekdays as day, index (index)}
                                        <DatePicker.HeadCell class="inspector-cal-head-cell">
                                            {day.slice(0, 2)}
                                        </DatePicker.HeadCell>
                                    {/each}
                                </DatePicker.GridRow>
                            </DatePicker.GridHead>
                            <DatePicker.GridBody>
                                {#each month.weeks as weekDates, weekIndex (weekIndex)}
                                    <DatePicker.GridRow>
                                        {#each weekDates as date (date.toString())}
                                            <DatePicker.Cell {date} month={month.value} class="inspector-cal-cell">
                                                <DatePicker.Day class="inspector-cal-day" />
                                            </DatePicker.Cell>
                                        {/each}
                                    </DatePicker.GridRow>
                                {/each}
                            </DatePicker.GridBody>
                        </DatePicker.Grid>
                    {/each}
                {/snippet}
            </DatePicker.Calendar>

            <div class="inspector-popover-section">
                <div class="inspector-popover-row">
                    <span class="inspector-popover-label">Time</span>
                    <div class="inspector-popover-time-controls">
                        <TimeInput
                            value={selectedTime}
                            onchange={handleTimeChange}
                            disabled={disabled || !selectedDate}
                            ariaLabel={timeAriaLabel}
                        />
                        {#if selectedTime && !disabled}
                            <button
                                type="button"
                                class="inspector-clear"
                                aria-label={`Clear ${timeAriaLabel}`}
                                onclick={() => handleTimeChange('')}
                            >
                                <span class="material-symbols-outlined" style="font-size: 14px;">close</span>
                            </button>
                        {/if}
                    </div>
                </div>
            </div>

            <div class="inspector-popover-footer">
                {#if selectedDate && !disabled}
                    <button
                        type="button"
                        class="inspector-popover-btn is-ghost"
                        onclick={() => {
                            handleClear();
                            open = false;
                        }}
                    >
                        Clear
                    </button>
                {/if}
                <DatePicker.Close class="inspector-popover-btn is-primary">
                    Done
                </DatePicker.Close>
            </div>
        </DatePicker.Content>
    </DatePicker.Portal>
</DatePicker.Root>
