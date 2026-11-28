<script lang="ts">
    import { untrack } from 'svelte';
    import { DateRangePicker, Portal, type DateRange } from 'bits-ui';
    import type { DateValue } from '@internationalized/date';
    import './picker.css';
    import TimeInput from '../inputs/TimeInput.svelte';
    import ToggleSwitch from '../inputs/ToggleSwitch.svelte';
    import {
        eventEndDate,
        eventEndDateInclusive,
        eventIsAllDay,
        eventStartDate,
        formatEventSchedule,
        toAllDay,
        toTimed,
        withDateRange,
        withEndTime,
        withStartTime,
        type EventDateFields,
    } from './eventDates';
    import { getFormattedTime, parseYmdToCalendarDate, todayCalendarDate } from './format';

    interface Props {
        event: EventDateFields;
        disabled?: boolean;
        onchange: (fields: EventDateFields) => void;
    }

    let { event, disabled = false, onchange }: Props = $props();

    function snapshotFields(source: EventDateFields): EventDateFields {
        return {
            startTime: source.startTime,
            endTime: source.endTime,
            isAllDay: source.isAllDay,
        };
    }

    function rangeForFields(fields: EventDateFields): DateRange {
        const startStr = eventStartDate(fields);
        const endStr = eventIsAllDay(fields)
            ? (eventEndDateInclusive(fields) ?? startStr)
            : (eventEndDate(fields) ?? startStr);
        return {
            start: parseYmdToCalendarDate(startStr),
            end: parseYmdToCalendarDate(endStr),
        };
    }

    let open = $state(false);
    let draft = $state<EventDateFields>(untrack(() => snapshotFields(event)));
    let rangeValue = $state<DateRange>(untrack(() => rangeForFields(event)));
    let calendarPlaceholder = $state<DateValue>(
        untrack(() => parseYmdToCalendarDate(eventStartDate(event)) ?? todayCalendarDate()),
    );

    $effect(() => {
        const next = snapshotFields(event);
        draft = next;
        rangeValue = rangeForFields(next);
    });

    const allDay = $derived(eventIsAllDay(draft));
    const displayText = $derived(formatEventSchedule(draft));
    const awaitingEnd = $derived(Boolean(rangeValue.start && !rangeValue.end));

    function handleOpenChange(nextOpen: boolean): void {
        open = nextOpen;
        if (nextOpen) {
            calendarPlaceholder = parseYmdToCalendarDate(eventStartDate(draft)) ?? todayCalendarDate();
            return;
        }
        const reset = snapshotFields(event);
        draft = reset;
        rangeValue = rangeForFields(reset);
    }

    function commit(fields: EventDateFields): void {
        draft = fields;
        rangeValue = rangeForFields(fields);
        onchange(fields);
    }

    function handleRangeChange(next: DateRange): void {
        rangeValue = next;
        if (disabled || !next.start || !next.end) return;
        commit(withDateRange(draft, next.start.toString(), next.end.toString()));
    }

    function handleAllDayToggle(checked: boolean): void {
        if (disabled) return;
        commit(checked ? toAllDay(draft) : toTimed(draft));
    }

    function handleStartTimeChange(value: string): void {
        if (disabled || !value) return;
        commit(withStartTime(draft, value));
    }

    function handleEndTimeChange(value: string): void {
        if (disabled || !value) return;
        commit(withEndTime(draft, value));
    }
</script>

<DateRangePicker.Root
    bind:open
    onOpenChange={handleOpenChange}
    bind:value={rangeValue}
    onValueChange={handleRangeChange}
    bind:placeholder={calendarPlaceholder}
    weekStartsOn={1}
    weekdayFormat="short"
    fixedWeeks={true}
    preventDeselect={true}
    closeOnRangeSelect={false}
    calendarLabel="Event dates"
    {disabled}
    class="inspector-picker-row"
>
    <DateRangePicker.Trigger
        class={`inspector-picker-trigger ${draft.startTime ? '' : 'is-empty'}`.trim()}
        aria-label="When"
        {disabled}
    >
        <span class="inspector-picker-label">{displayText}</span>
    </DateRangePicker.Trigger>

    <Portal>
        <DateRangePicker.Content
            sideOffset={6}
            align="start"
            class="inspector-date-popover"
            data-inspector-popover={open ? '' : undefined}
        >
            <DateRangePicker.Calendar>
                {#snippet children({ months, weekdays })}
                    <DateRangePicker.Header class="inspector-cal-header">
                        <DateRangePicker.PrevButton class="inspector-cal-nav" aria-label="Previous month">
                            <span class="material-symbols-outlined" aria-hidden="true">chevron_left</span>
                        </DateRangePicker.PrevButton>
                        <DateRangePicker.Heading class="inspector-cal-heading" />
                        <DateRangePicker.NextButton class="inspector-cal-nav" aria-label="Next month">
                            <span class="material-symbols-outlined" aria-hidden="true">chevron_right</span>
                        </DateRangePicker.NextButton>
                    </DateRangePicker.Header>

                    {#each months as month (month.value.toString())}
                        <DateRangePicker.Grid class="inspector-cal-grid">
                            <DateRangePicker.GridHead>
                                <DateRangePicker.GridRow>
                                    {#each weekdays as day, index (index)}
                                        <DateRangePicker.HeadCell class="inspector-cal-head-cell">
                                            {day.slice(0, 2)}
                                        </DateRangePicker.HeadCell>
                                    {/each}
                                </DateRangePicker.GridRow>
                            </DateRangePicker.GridHead>
                            <DateRangePicker.GridBody>
                                {#each month.weeks as weekDates, weekIndex (weekIndex)}
                                    <DateRangePicker.GridRow>
                                        {#each weekDates as date (date.toString())}
                                            <DateRangePicker.Cell {date} month={month.value} class="inspector-cal-cell">
                                                <DateRangePicker.Day class="inspector-cal-day" />
                                            </DateRangePicker.Cell>
                                        {/each}
                                    </DateRangePicker.GridRow>
                                {/each}
                            </DateRangePicker.GridBody>
                        </DateRangePicker.Grid>
                    {/each}
                {/snippet}
            </DateRangePicker.Calendar>

            <div class="inspector-popover-section">
                <div class="inspector-popover-row">
                    <span class="inspector-popover-label">All day</span>
                    <ToggleSwitch
                        checked={allDay}
                        onchange={handleAllDayToggle}
                        ariaLabel="All day event"
                        {disabled}
                    />
                </div>

                {#if !allDay}
                    <div class="inspector-popover-row">
                        <span class="inspector-popover-label">Start time</span>
                        <TimeInput
                            value={getFormattedTime(draft.startTime)}
                            onchange={handleStartTimeChange}
                            {disabled}
                            ariaLabel="Start time"
                        />
                    </div>
                    <div class="inspector-popover-row">
                        <span class="inspector-popover-label">End time</span>
                        <TimeInput
                            value={getFormattedTime(draft.endTime)}
                            onchange={handleEndTimeChange}
                            {disabled}
                            ariaLabel="End time"
                        />
                    </div>
                {/if}
            </div>

            <div class="inspector-popover-footer">
                {#if awaitingEnd}
                    <span class="inspector-popover-hint">Select end date</span>
                {/if}
                <DateRangePicker.Close class="inspector-popover-btn is-primary">
                    Done
                </DateRangePicker.Close>
            </div>
        </DateRangePicker.Content>
    </Portal>
</DateRangePicker.Root>
