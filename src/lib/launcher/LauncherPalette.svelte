<script lang="ts">
    import { onMount } from 'svelte';
    import { commands, events, safeInvoke, useAutoQuery } from '$lib/safeInvoke.svelte';
    import { store } from '$lib/store.svelte';
    import { Routes } from '$lib/routes';
    import { eventInstances } from '$lib/queries';
    import { useNow } from '$lib/useNow.svelte';
    import { formatHhMm, minutesSinceMidnight, parseLocalYmd, ymd } from '$lib/time';
    import type { AppTask, AppTaskFilter, AppTaskSort, EventInstance, FilterGroup } from '$lib/domain';
    import { emptyFilterGroup, filterGroup } from '$lib/domain';
    import { formatQuickCaptureDue, parseQuickCapture, type QuickCapturePreview } from './quickCapture';

    const DAY_CHECK_INTERVAL_MS = 60_000;

    const now = useNow(DAY_CHECK_INTERVAL_MS);
    let currentNow = $derived(new Date(now.ms));
    let todayIso = $derived(ymd(currentNow));
    let today = $derived(parseLocalYmd(todayIso));
    let todayRange = $derived({ startDate: todayIso, endDate: todayIso });

    function refreshToday(): void {
        currentNow = new Date();
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
    let activePreview = $derived(
        text.trim() === ''
            ? null
            : parseQuickCapture(text, currentNow, calendars, store.settings?.default_task_list),
    );
    let createTitle = $derived(activePreview?.title.trim() || text.trim());
    let createParts = $derived(activePreview ? previewParts(activePreview) : []);
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
            remoteCollectionId: current.listQuery !== null ? current.list?.id : undefined,
        };

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
        return formatQuickCaptureDue(value.due, value.hasTime, today);
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
