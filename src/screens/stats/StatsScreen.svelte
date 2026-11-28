<script lang="ts">
    import { commands, describeAppError, useAutoQuery } from '../../lib/safeInvoke.svelte';
    import { allEvents, allTasks } from '../../lib/queries';
    import { useNow } from '../../lib/useNow.svelte';
    import BentoCard from './BentoCard.svelte';
    import ActivityChart from './ActivityChart.svelte';
    import PeakHoursChart from './PeakHoursChart.svelte';
    import TopTasksCard from './TopTasksCard.svelte';
    import TagsCard from './TagsCard.svelte';
    import StatusCard from './StatusCard.svelte';
    import PriorityCard from './PriorityCard.svelte';
    import { computeStats, formatDuration } from './metrics';
    import './stats.css';

    const tasksQuery = useAutoQuery(commands.queryTasks, allTasks);
    const eventsQuery = useAutoQuery(commands.queryEvents, allEvents);
    const historyQuery = useAutoQuery(commands.getStopwatchHistory);
    const tagsQuery = useAutoQuery(commands.listTags);

    const now = useNow(60_000);

    const stats = $derived(
        computeStats({
            tasks: tasksQuery.data ?? [],
            events: eventsQuery.data ?? [],
            history: historyQuery.data ?? [],
            tags: tagsQuery.data ?? [],
            now: now.value,
        }),
    );

    const isLoading = $derived(
        tasksQuery.isLoading || eventsQuery.isLoading || historyQuery.isLoading || tagsQuery.isLoading,
    );

    const queryError = $derived(
        tasksQuery.error ?? eventsQuery.error ?? historyQuery.error ?? tagsQuery.error ?? null,
    );
</script>

<main class="main stats-main">
    <div class="stats-scroll">
        <header class="stats-hd">
            <div class="stats-title">
                <span class="bracket">[</span>
                <span class="stats-title-label">stats</span>
                <span class="bracket">]</span>
                <span class="stats-subtitle">forever · all-time overview</span>
            </div>
            <div class="stats-hd-meta">
                {#if isLoading}
                    <span class="stats-pill">syncing…</span>
                {/if}
                <span class="stats-pill">{stats.activeDaysTotal} active days</span>
                <span class="stats-pill">{stats.totalTasks} tasks · {stats.totalEvents} events</span>
            </div>
        </header>

        {#if queryError}
            <div class="stats-error" role="alert">
                Failed to load some statistics: {describeAppError(queryError)}
            </div>
        {/if}

        <div class="bento-grid">
            <BentoCard title="total focus time" span="col-3" testId="kpi-focus-time">
                <div class="kpi-value">{formatDuration(stats.totalTrackedSeconds)}</div>
                <div class="kpi-sub">
                    {stats.sessionCount} {stats.sessionCount === 1 ? 'session' : 'sessions'} · {stats.avgSessionMinutes}m avg
                </div>
            </BentoCard>

            <BentoCard title="tasks completed" span="col-3" testId="kpi-tasks-completed">
                {#snippet badge()}
                    <span class="kpi-badge">{stats.completionRatePct}%</span>
                {/snippet}
                <div class="kpi-value">{stats.doneTasks}</div>
                <div class="kpi-progress">
                    <div class="bar-track">
                        <div class="bar-fill is-success" style:width={`${stats.completionRatePct}%`}></div>
                    </div>
                </div>
                <div class="kpi-sub">
                    of {stats.totalTasks} total · {stats.openTasks + stats.nextUpTasks} active
                </div>
            </BentoCard>

            <BentoCard title="current streak" span="col-3" testId="kpi-streak">
                <div class="kpi-value">
                    {stats.currentStreakDays}<span class="kpi-unit">d</span>
                </div>
                <div class="kpi-sub">
                    Longest: {stats.longestStreakDays}d · {stats.activeDaysTotal} active days
                </div>
            </BentoCard>

            <BentoCard title="focus ratio" span="col-3" testId="kpi-focus-ratio">
                <div class="kpi-value">
                    {stats.focusRatioPct}<span class="kpi-unit">%</span>
                </div>
                <div class="kpi-sub">
                    Break: {formatDuration(stats.totalBreakSeconds)} · {stats.totalEvents} events ({stats.scheduledTaskBlocks} blocks)
                </div>
            </BentoCard>

            <ActivityChart days={stats.dailyActivity} />

            <PeakHoursChart
                buckets={stats.hourlyActivity}
                peakHour={stats.peakHour}
                peakHourLabel={stats.peakHourLabel}
            />

            <TopTasksCard tasks={stats.topTasksByTime} />

            <TagsCard tags={stats.topTags} totalKnownTags={stats.totalKnownTags} />

            <StatusCard
                totalTasks={stats.totalTasks}
                doneTasks={stats.doneTasks}
                nextUpTasks={stats.nextUpTasks}
                openTasks={stats.openTasks}
                cancelledTasks={stats.cancelledTasks}
            />

            <PriorityCard
                priorities={stats.priorityBreakdown}
                totalTasks={stats.totalTasks}
            />
        </div>
    </div>
</main>
