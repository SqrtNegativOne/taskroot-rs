<script lang="ts">
    import { allTasks } from '../../lib/queries';
    import { commands, useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { store } from '../../lib/store.svelte';
    import { buildWrapDaySummary } from './wrapData';
    import WrapTimeline from './WrapTimeline.svelte';
    import WrapReflectionPane from './WrapReflectionPane.svelte';

    interface Props {
        wrapDate: Date;
        tomorrow: Date;
        isYesterday: boolean;
    }

    let { wrapDate, tomorrow, isYesterday }: Props = $props();

    const tasksQuery = useAutoQuery(commands.queryTasks, allTasks);
    const historyQuery = useAutoQuery(commands.getStopwatchHistory);
    const calendarsQuery = useAutoQuery(commands.getActiveCalendars);

    let tasks = $derived(tasksQuery.data ?? []);
    let history = $derived(historyQuery.data ?? []);
    let calendars = $derived(calendarsQuery.data ?? []);

    let summary = $derived(
        buildWrapDaySummary(tasks, history, wrapDate, store.settings?.earliest_wake_up_time),
    );
</script>

<main class="wrap-main">
    <WrapTimeline {summary} {isYesterday} />
    <WrapReflectionPane
        {wrapDate}
        {tomorrow}
        {tasks}
        completedTasks={summary.completedTasks}
        {calendars}
    />
</main>
