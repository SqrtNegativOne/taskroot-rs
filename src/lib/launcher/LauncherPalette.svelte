<script lang="ts">
    import { onMount } from 'svelte';
    import { commands, events, safeInvoke, useAutoQuery } from '$lib/safeInvoke.svelte';
    import { store } from '$lib/store.svelte';
    import { Routes } from '$lib/routes';
    import { eventInstances } from '$lib/queries';
    import { useNow } from '$lib/useNow.svelte';
    import { parseLocalYmd, ymd } from '$lib/time';
    import type { AppTaskFilter, AppTaskSort, FilterGroup } from '$lib/domain';
    import { emptyFilterGroup, filterGroup } from '$lib/domain';
    import { parseQuickCapture } from './quickCapture';
    import {
        buildPaletteRows,
        buildTaskFromQuickCapture,
        filterTodayTasks,
        formatEventDetail,
        formatPreviewParts,
        formatTaskDetail,
        matchRouteCommand,
        type PaletteRow,
    } from './paletteRows';
    import './launcherPalette.css';

    const DAY_CHECK_INTERVAL_MS = 60_000;
    const OPEN_TASK_FILTERS: FilterGroup<AppTaskFilter> = filterGroup([
        { column: 'status', operator: 'is not', value: ['done'] },
    ]);

    const now = useNow(DAY_CHECK_INTERVAL_MS);
    let currentNow = $derived(new Date(now.ms));
    let todayIso = $derived(ymd(currentNow));
    let today = $derived(parseLocalYmd(todayIso));
    let todayRange = $derived({ startDate: todayIso, endDate: todayIso });

    function refreshToday(): void {
        currentNow = new Date();
    }

    let text = $state('');
    let selected = $state(0);
    let inputEl = $state<HTMLInputElement | null>(null);

    const calendarsQuery = useAutoQuery(commands.getActiveCalendars);
    const eventsQuery = useAutoQuery(commands.queryEventInstances, () =>
        eventInstances(todayRange, emptyFilterGroup(), ''),
    );
    const tasksQuery = useAutoQuery(commands.queryTasks, (): [FilterGroup<AppTaskFilter>, AppTaskSort[], string] => [
        OPEN_TASK_FILTERS,
        [],
        '',
    ]);

    let calendars = $derived(calendarsQuery.data ?? []);
    let todayEvents = $derived(eventsQuery.data ?? []);
    let todayTasks = $derived(filterTodayTasks(tasksQuery.data ?? [], todayIso));
    let rows = $derived(buildPaletteRows(text, todayEvents, todayTasks));

    let activeIndex = $derived(rows.length === 0 ? 0 : Math.min(selected, rows.length - 1));
    let activePreview = $derived(
        text.trim() === ''
            ? null
            : parseQuickCapture(text, currentNow, calendars, store.settings?.default_task_list),
    );
    let createTitle = $derived(activePreview?.title.trim() || text.trim());
    let createParts = $derived(activePreview ? formatPreviewParts(activePreview, today) : []);
    let createBlocked = $derived(
        activePreview !== null &&
            (activePreview.listError !== null || activePreview.title.trim() === ''),
    );

    $effect(() => {
        void text;
        selected = 0;
    });

    onMount(() => {
        inputEl?.focus();
        void safeInvoke(commands.resizeLauncher(720, 520));
        const handleVisibility = () => {
            if (document.visibilityState === 'visible') refreshToday();
        };
        window.addEventListener('focus', refreshToday);
        document.addEventListener('visibilitychange', handleVisibility);
        return () => {
            window.removeEventListener('focus', refreshToday);
            document.removeEventListener('visibilitychange', handleVisibility);
        };
    });

    async function createFromPreview(): Promise<void> {
        const current = activePreview;
        if (current === null || current.listError !== null || current.title.trim() === '') return;

        const task = buildTaskFromQuickCapture(
            current,
            Intl.DateTimeFormat().resolvedOptions().timeZone,
        );
        const result = await store.mutate('Failed to create task', () => store.addTask(task));
        if (result.isOk()) await hide();
    }

    async function hide(): Promise<void> {
        text = '';
        await safeInvoke(commands.hideLauncher());
    }

    async function navigate(route: string): Promise<void> {
        await events.launcherNavigate.emit(route);
        await hide();
    }

    function moveSelection(delta: number): void {
        const count = rows.length;
        if (count === 0) return;
        selected = (activeIndex + delta + count) % count;
    }

    async function activate(row: PaletteRow): Promise<void> {
        if (row.kind === 'create') {
            await createFromPreview();
            return;
        }
        await navigate(Routes.HOME);
    }

    async function handleKeydown(event: KeyboardEvent): Promise<void> {
        if (event.key === 'Escape') {
            event.preventDefault();
            await hide();
            return;
        }
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            moveSelection(event.key === 'ArrowDown' ? 1 : -1);
            return;
        }
        if (event.key !== 'Enter') return;

        event.preventDefault();
        const command = matchRouteCommand(text);
        if (command !== undefined) {
            await navigate(command);
            return;
        }
        const row = rows[activeIndex];
        if (row !== undefined) await activate(row);
    }
</script>

<div class="palette">
    <input
        bind:this={inputEl}
        class="palette-input"
        type="text"
        placeholder="Type a task or command..."
        autofocus
        bind:value={text}
        onkeydown={handleKeydown}
    />

    <div class="palette-results">
        {#if text.trim() === ''}
            <div class="section-label">Today</div>
        {/if}

        {#if rows.length === 0}
            <p class="empty">Nothing on today's agenda.</p>
        {:else}
            {#each rows as row, index (row.key)}
                {#if row.kind === 'create'}
                    <button
                        type="button"
                        class="row create-row"
                        class:selected={index === activeIndex}
                        class:blocked={createBlocked}
                        onclick={() => void createFromPreview()}
                    >
                        <span class="row-kind">New task</span>
                        <span class="row-body">
                            <span class="row-title">{createTitle}</span>
                            {#if createParts.length > 0}
                                <span class="row-detail">{createParts.join(' · ')}</span>
                            {/if}
                            {#if activePreview?.listError}
                                <span class="row-error">{activePreview.listError}</span>
                            {/if}
                        </span>
                    </button>
                {:else if row.kind === 'event'}
                    <button
                        type="button"
                        class="row"
                        class:selected={index === activeIndex}
                        onclick={() => void navigate(Routes.HOME)}
                    >
                        <span class="row-kind">Event</span>
                        <span class="row-body">
                            <span class="row-title">{row.event.title}</span>
                            <span class="row-detail">{formatEventDetail(row.event)}</span>
                        </span>
                    </button>
                {:else}
                    <button
                        type="button"
                        class="row"
                        class:selected={index === activeIndex}
                        onclick={() => void navigate(Routes.HOME)}
                    >
                        <span class="row-kind">Task</span>
                        <span class="row-body">
                            <span class="row-title">{row.task.title}</span>
                            <span class="row-detail">{formatTaskDetail(row.task, todayIso)}</span>
                        </span>
                    </button>
                {/if}
            {/each}
        {/if}
    </div>
</div>
