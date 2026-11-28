<script lang="ts">
    import {
        DEFAULT_TASK_PRIORITY,
        taskDateOccursOnDay,
        type AppTask,
        type TaskPatch,
    } from '../../lib/domain';
    import { taskDates } from '../../lib/queries';
    import { commands, useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { store } from '../../lib/store.svelte';
    import { toWallClockParts } from '../../lib/time';
    import SearchBar from '../../components/SearchBar.svelte';
    import { formatWrapDateLabel } from './wrapWindow';

    interface Props {
        tasks: readonly AppTask[];
        tomorrowDate: Date;
        tomorrowYmd: string;
    }

    let { tasks, tomorrowDate, tomorrowYmd }: Props = $props();

    let search = $state('');
    let newTaskTitle = $state('');

    const tomorrowMarkersQuery = useAutoQuery(
        commands.queryTaskDates,
        () => taskDates({ startDate: tomorrowYmd, endDate: tomorrowYmd }),
    );

    let tomorrowMarkers = $derived(
        (tomorrowMarkersQuery.data ?? []).filter((marker) => taskDateOccursOnDay(marker, tomorrowDate)),
    );
    let tomorrowSlatedIds = $derived(new Set(tomorrowMarkers.map((marker) => marker.taskId)));
    let tomorrowStartIds = $derived(
        new Set(
            tomorrowMarkers
                .filter((marker) => marker.kind === 'start')
                .map((marker) => marker.taskId),
        ),
    );

    let slatedTomorrow = $derived(
        tasks.filter(
            (task) =>
                task.status !== 'done'
                && task.status !== 'cancelled'
                && tomorrowSlatedIds.has(task.id),
        ),
    );

    let slatedIds = $derived(new Set(slatedTomorrow.map((task) => task.id)));

    let openTodoCandidates = $derived.by(() => {
        const needle = search.trim().toLowerCase();
        return tasks.filter((task) => {
            if (task.status !== 'todo' || slatedIds.has(task.id)) return false;
            if (!needle) return true;
            const inTitle = task.title.toLowerCase().includes(needle);
            const inTags = (task.tags ?? []).some((tag) => tag.name.toLowerCase().includes(needle));
            return inTitle || inTags;
        });
    });

    function promoteToTomorrow(task: AppTask): void {
        void store.mutate('Failed to schedule task for tomorrow', () =>
            store.updateTask(task.id, [
                { field: 'status', value: 'next-up' },
                { field: 'start', value: tomorrowYmd },
            ]),
        );
    }

    function unplanFromTomorrow(task: AppTask): void {
        const patches: TaskPatch[] = tomorrowStartIds.has(task.id)
            ? [
                { field: 'status', value: 'todo' },
                { field: 'start', value: null },
            ]
            : [{ field: 'status', value: 'todo' }];
        void store.mutate('Failed to remove task from tomorrow', () =>
            store.updateTask(task.id, patches),
        );
    }

    async function handleQuickAdd(e: SubmitEvent): Promise<void> {
        e.preventDefault();
        const title = newTaskTitle.trim();
        if (!title) return;

        const newTask: AppTask = {
            id: crypto.randomUUID(),
            title,
            status: 'next-up',
            priority: DEFAULT_TASK_PRIORITY,
            start: tomorrowYmd,
        };
        const result = await store.mutate('Failed to create task', () => store.addTask(newTask));
        if (result.isOk()) {
            newTaskTitle = '';
        }
    }
</script>

<section class="wrap-card" aria-label="Plan for Tomorrow">
    <header class="wrap-card-hd">
        <h3 class="wrap-card-title">Plan for tomorrow</h3>
        <span class="wrap-card-subtitle">{formatWrapDateLabel(tomorrowDate)}</span>
    </header>

    <form class="wrap-inline-form" onsubmit={handleQuickAdd}>
        <input
            type="text"
            class="wrap-input"
            bind:value={newTaskTitle}
            placeholder="Quick-add task for tomorrow..."
            aria-label="New task for tomorrow"
        />
        <button type="submit" class="wrap-btn is-accent" disabled={!newTaskTitle.trim()}>
            Add
        </button>
    </form>

    <div class="wrap-subsection">
        <div class="wrap-sub-hd">
            <span>Slated for tomorrow</span>
            <span>{slatedTomorrow.length}</span>
        </div>
        {#if slatedTomorrow.length === 0}
            <p class="wrap-empty-note">Nothing planned for tomorrow yet.</p>
        {:else}
            <div class="wrap-task-list">
                {#each slatedTomorrow as task (task.id)}
                    <div class="wrap-task-row">
                        <div class="wrap-task-info">
                            <span class="wrap-task-title">{task.title}</span>
                            <span class="wrap-task-meta">
                                {task.status === 'next-up' ? 'next up' : 'due'}
                            </span>
                        </div>
                        <div class="wrap-task-actions">
                            {#if task.status !== 'next-up'}
                                <button
                                    type="button"
                                    class="wrap-btn is-accent"
                                    onclick={() => promoteToTomorrow(task)}
                                >
                                    Next up
                                </button>
                            {:else}
                                <button
                                    type="button"
                                    class="wrap-btn"
                                    onclick={() => unplanFromTomorrow(task)}
                                    title="Move back to todo"
                                >
                                    Unplan
                                </button>
                            {/if}
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>

    <div class="wrap-subsection">
        <div class="wrap-sub-hd">
            <span>Pick from backlog</span>
            <span>{openTodoCandidates.length}</span>
        </div>
        <SearchBar value={search} onchange={(val) => { search = val; }} />
        {#if openTodoCandidates.length === 0}
            <p class="wrap-empty-note">No matching open tasks.</p>
        {:else}
            <div class="wrap-task-list wrap-task-scroll">
                {#each openTodoCandidates as task (task.id)}
                    <button
                        type="button"
                        class="wrap-task-row is-interactive"
                        onclick={() => promoteToTomorrow(task)}
                    >
                        <div class="wrap-task-info">
                            <span class="wrap-task-title">{task.title}</span>
                            {#if task.due}
                                {@const dueDate = toWallClockParts(task.due, task.timezone)?.date}
                                {#if dueDate}
                                    <span class="wrap-task-meta">due {dueDate}</span>
                                {/if}
                            {/if}
                        </div>
                        <span class="wrap-btn is-accent">+ Tomorrow</span>
                    </button>
                {/each}
            </div>
        {/if}
    </div>
</section>
