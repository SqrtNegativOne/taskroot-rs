<script lang="ts">
    import { onDestroy, onMount } from 'svelte';
    import { emit } from '@tauri-apps/api/event';
    import { safeInvoke, useAutoQuery } from '$lib/safeInvoke.svelte';
    import { describeError, store } from '$lib/store.svelte';
    import { notifyError } from '$lib/toast';
    import { LAUNCHER_NAVIGATE } from '$lib/ipcEvents';
    import { Routes } from '$lib/routes';
    import { eventInstances } from '$lib/queries';
    import { createDebouncer, createStaleGuard } from '$lib/asyncState.svelte';
    import { useNow } from '$lib/useNow.svelte';
    import { addDays, formatHhMm, minutesSinceMidnight, parseLocalYmd, sameDay, ymd } from '$lib/time';
    import type { AppTask, AppTaskFilter, EventInstance, FilterGroup, QuickCapturePreview } from '$lib/domain';
    import { emptyFilterGroup, filterGroup } from '$lib/domain';

    const PARSE_DEBOUNCE_MS = 160;
    const DAY_CHECK_INTERVAL_MS = 60_000;

    const now = useNow(DAY_CHECK_INTERVAL_MS);
    let todayIso = $derived(ymd(now.value));
    let today = $derived(parseLocalYmd(todayIso));
    let todayRange = $derived({ startDate: todayIso, endDate: todayIso });

    function refreshToday(): void {
        todayIso = ymd(new Date());
    }

    const OPEN_TASK_FILTERS: FilterGroup<AppTaskFilter> = filterGroup([
        { column: 'status', operator: 'is not', value: ['done'] },
    ]);

    const ROUTE_COMMANDS: Record<string, string> = {
        home: Routes.HOME,
        plan: Routes.HOME,
        do: Routes.DO,
        tags: Routes.TAGS,
        settings: Routes.SETTINGS,
    };

    let text = $state('');
    let preview = $state<QuickCapturePreview | null>(null);
    let previewText = $state('');
    let selected = $state(0);
    let inputEl = $state<HTMLInputElement | null>(null);

    const guard = createStaleGuard();

    const eventsQuery = useAutoQuery<EventInstance[]>('query_event_instances', () =>
        eventInstances(todayRange, emptyFilterGroup(), ''),
    );
    const tasksQuery = useAutoQuery<AppTask[]>('query_tasks', () => ({
        filters: OPEN_TASK_FILTERS,
        sort: [],
        query: '',
    }));

    let todayEvents = $derived(eventsQuery.data ?? []);
    let todayTasks = $derived(
        (tasksQuery.data ?? []).filter(
            (task) => task.start?.startsWith(todayIso) === true || task.due?.startsWith(todayIso) === true,
        ),
    );

    type Row =
        | { key: 'create'; kind: 'create' }
        | { key: string; kind: 'event'; event: EventInstance }
        | { key: string; kind: 'task'; task: AppTask };

    let rows = $derived.by<Row[]>(() => {
        const value = text.trim();
        const typed = value !== '';
        const needle = value.toLowerCase();
        const result: Row[] = [];

        if (typed) result.push({ key: 'create', kind: 'create' });

        for (const event of todayEvents) {
            if (!typed || event.title.toLowerCase().includes(needle)) {
                result.push({ key: `event:${event.occurrenceKey}`, kind: 'event', event });
            }
        }

        for (const task of todayTasks) {
            if (!typed || task.title.toLowerCase().includes(needle)) {
                result.push({ key: `task:${task.id}`, kind: 'task', task });
            }
        }

        return result;
    });

    let activeIndex = $derived(rows.length === 0 ? 0 : Math.min(selected, rows.length - 1));
    let previewPending = $derived(text.trim() !== '' && previewText !== text);
    let activePreview = $derived(preview !== null && previewText === text ? preview : null);
    let createTitle = $derived(activePreview?.title.trim() || text.trim());
    let createParts = $derived(activePreview ? previewParts(activePreview) : []);
    let createBlocked = $derived(
        activePreview !== null &&
            (activePreview.listError !== null || activePreview.title.trim() === ''),
    );

    const debouncer = createDebouncer<string>((value) => {
        void runParse(value);
    }, PARSE_DEBOUNCE_MS);

    $effect(() => {
        const value = text;
        if (value.trim() === '') {
            debouncer.cancel();
            guard.invalidate();
            preview = null;
            previewText = '';
            return;
        }
        guard.invalidate();
        debouncer.schedule(value);
    });

    $effect(() => {
        void text;
        selected = 0;
    });

    onMount(() => {
        inputEl?.focus();
        void safeInvoke('resize_launcher', { width: 720, height: 520 });
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

    onDestroy(() => {
        debouncer.cancel();
        guard.invalidate();
    });

    async function runParse(value: string): Promise<QuickCapturePreview | null> {
        const isCurrent = guard.begin();
        const result = await safeInvoke<QuickCapturePreview>('parse_quick_capture', { text: value });
        if (!isCurrent()) return null;

        if (result.isErr()) {
            preview = null;
            previewText = value;
            notifyError(`Could not parse quick capture: ${describeError(result.error)}`);
            return null;
        }

        preview = result.value;
        previewText = value;
        return result.value;
    }

    async function createFromPreview(): Promise<void> {
        const value = text.trim();
        if (value === '') return;

        const current = preview !== null && previewText === text ? preview : await runParse(value);
        if (current === null) return;
        if (current.listError !== null || current.title.trim() === '') return;

        const task: AppTask = {
            id: crypto.randomUUID(),
            title: current.title,
            status: 'todo',
            priority: current.priority ?? undefined,
            tags: current.tags.map((name) => ({ id: crypto.randomUUID(), name })),
            est: current.estimateMinutes ?? undefined,
            due: current.due ?? undefined,
            timezone: current.hasTime ? Intl.DateTimeFormat().resolvedOptions().timeZone : undefined,
            remoteCollectionId: current.list?.id,
        };

        const result = await store.mutate('Failed to create task', () => store.addTask(task));
        if (result.isOk()) await hide();
    }

    async function hide(): Promise<void> {
        debouncer.cancel();
        guard.invalidate();
        text = '';
        preview = null;
        previewText = '';
        await safeInvoke('hide_launcher');
    }

    async function navigate(route: string): Promise<void> {
        await emit(LAUNCHER_NAVIGATE, route);
        await hide();
    }

    function moveSelection(delta: number): void {
        const count = rows.length;
        if (count === 0) return;
        selected = (activeIndex + delta + count) % count;
    }

    async function activate(row: Row): Promise<void> {
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
        if (event.key === 'ArrowDown') {
            event.preventDefault();
            moveSelection(1);
            return;
        }
        if (event.key === 'ArrowUp') {
            event.preventDefault();
            moveSelection(-1);
            return;
        }
        if (event.key !== 'Enter') return;

        event.preventDefault();
        const command = ROUTE_COMMANDS[text.trim().toLowerCase()];
        if (command !== undefined) {
            await navigate(command);
            return;
        }
        const row = rows[activeIndex];
        if (row !== undefined) await activate(row);
    }

    function previewParts(value: QuickCapturePreview): string[] {
        const parts: string[] = [];
        const due = dueLabel(value);
        if (due !== null) parts.push(due);
        if (value.tags.length > 0) parts.push(value.tags.map((tag) => `#${tag}`).join(' '));
        if (value.priority !== null) parts.push(`P${value.priority}`);
        if (value.estimateMinutes !== null) parts.push(`${value.estimateMinutes} min`);
        if (value.list !== null) parts.push(value.list.summary);
        return parts;
    }

    function dueLabel(value: QuickCapturePreview): string | null {
        if (value.due === null) return null;

        const [datePart, timePart] = value.due.split('T');
        const day = relativeDay(datePart);
        if (!value.hasTime || timePart === undefined) return day;

        const hours = Number(timePart.slice(0, 2));
        const minutes = Number(timePart.slice(3, 5));
        return `${day} ${formatHhMm(hours * 60 + minutes)}`;
    }

    function relativeDay(datePart: string): string {
        const date = parseLocalYmd(datePart);
        if (sameDay(date, today)) return 'Today';
        if (sameDay(date, addDays(today, 1))) return 'Tomorrow';
        return date.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' });
    }

    function eventDetail(event: EventInstance): string {
        if (event.timing.kind === 'allDay') return 'All day';
        const start = new Date(event.timing.start);
        const end = new Date(event.timing.end);
        return `${formatHhMm(minutesSinceMidnight(start))}–${formatHhMm(minutesSinceMidnight(end))}`;
    }

    function taskDetail(task: AppTask): string {
        return task.due?.startsWith(todayIso) === true ? 'Due today' : 'Planned today';
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
                            {#if previewPending}
                                <span class="row-detail">Parsing...</span>
                            {:else if createParts.length > 0}
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
                            <span class="row-detail">{eventDetail(row.event)}</span>
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
                            <span class="row-detail">{taskDetail(row.task)}</span>
                        </span>
                    </button>
                {/if}
            {/each}
        {/if}
    </div>
</div>

<style>
    .palette {
        display: flex;
        flex-direction: column;
        height: 100vh;
        width: 100vw;
        padding: 12px;
        gap: 10px;
        color: var(--fg);
        background: var(--bg-surface);
        border: 1px solid var(--border-strong);
        border-radius: 10px;
        box-shadow: var(--shadow-btn-hover);
        overflow: hidden;
    }

    .palette-input {
        flex: 0 0 auto;
        padding: 10px 12px;
        font: inherit;
        font-size: 1.15em;
        color: inherit;
        background: var(--bg-highlight);
        border: 1px solid var(--border);
        border-radius: 6px;
        outline: none;
    }

    .palette-input:focus {
        border-color: var(--accent, var(--border-strong));
    }

    .palette-results {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 2px;
        overflow-y: auto;
    }

    .section-label {
        padding: 4px 8px;
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: var(--fg-dim);
    }

    .empty {
        margin: 0;
        padding: 12px 8px;
        color: var(--fg-muted);
        font-size: 13px;
    }

    .row {
        display: flex;
        align-items: flex-start;
        gap: 10px;
        width: 100%;
        padding: 8px 10px;
        text-align: left;
        font: inherit;
        color: inherit;
        background: transparent;
        border: 1px solid transparent;
        border-radius: 6px;
        cursor: pointer;
    }

    .row.selected,
    .row:hover {
        background: var(--bg-surface-hover);
        border-color: var(--border);
    }

    .create-row.blocked {
        opacity: 0.7;
    }

    .row-kind {
        flex: 0 0 auto;
        min-width: 64px;
        padding-top: 2px;
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--fg-muted);
    }

    .row-body {
        display: flex;
        flex-direction: column;
        gap: 2px;
        min-width: 0;
    }

    .row-title {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .row-detail {
        font-size: 12px;
        color: var(--fg-dim);
    }

    .row-error {
        font-size: 12px;
        color: var(--tag-red);
    }
</style>
