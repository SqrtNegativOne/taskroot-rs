<script lang="ts">
    let {
        priority,
        isDoneOrChecking,
        isActive,
        onclick,
        title = "Toggle Done",
        ariaLabel,
        size = "md",
    }: {
        priority?: number | string;
        isDoneOrChecking: boolean;
        isActive?: boolean;
        onclick: (e: MouseEvent) => void;
        title?: string;
        ariaLabel?: string;
        size?: "sm" | "md";
    } = $props();

    let priorityLevel = $derived(priority === undefined ? undefined : Number(priority));
</script>

<button
    type="button"
    class="task-circle"
    class:is-sm={size === "sm"}
    class:pri-bg-0={priorityLevel === 0}
    class:pri-bg-1={priorityLevel === 1}
    class:pri-bg-2={priorityLevel === 2}
    class:pri-bg-3={priorityLevel === 3}
    class:pri-bg-4={priorityLevel === 4}
    style="padding: 0; font: inherit; color: inherit; cursor: pointer;"
    {title}
    aria-label={ariaLabel ?? (priority !== undefined ? `Priority ${priority.toString()}` : 'Toggle Done')}
    onpointerdown={(e) => { e.stopPropagation(); }}
    {onclick}
>
    {#if isActive}
        <svg
            class="task-circle-play"
            viewBox="0 0 24 24"
            fill="currentColor"
            style="width: 12px; height: 12px; color: var(--on-priority);"
        >
            <path d="M8 5v14l11-7z" />
        </svg>
    {:else if isDoneOrChecking}
        <svg
            class="task-circle-check"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="3"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <polyline points="4 12 9 17 20 6"></polyline>
        </svg>
    {/if}
</button>

<style>
    .task-circle {
        width: 18px;
        height: 18px;
        min-width: 18px;
        min-height: 18px;
        aspect-ratio: 1;
        border-radius: 50%;
        border: 2px solid var(--circle-border, var(--border));
        background-color: var(--circle-fill, transparent);
        margin-right: 8px;
        margin-top: 1px;
        flex-shrink: 0;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s ease;
    }

    .task-circle:hover {
        border-color: var(--circle-border, var(--accent));
    }

    .task-circle.is-sm {
        width: 13px;
        height: 13px;
        min-width: 13px;
        min-height: 13px;
        border-width: 1.5px;
        margin-right: 0;
        margin-top: 0;
    }

    .task-circle-check {
        width: 12px;
        height: 12px;
        color: var(--on-priority);
        stroke-dasharray: 24;
        stroke-dashoffset: 24;
        animation: dash 0.2s cubic-bezier(0.65, 0, 0.45, 1) forwards;
    }

    .task-circle.is-sm .task-circle-check {
        width: 8px;
        height: 8px;
    }

    @keyframes dash {
        to { stroke-dashoffset: 0; }
    }

    .pri-bg-0 { --circle-fill: var(--priority-fill-0); --circle-border: var(--priority-0); }
    .pri-bg-1 { --circle-fill: var(--priority-fill-1); --circle-border: var(--priority-1); }
    .pri-bg-2 { --circle-fill: var(--priority-fill-2); --circle-border: var(--priority-2); }
    .pri-bg-3 { --circle-fill: var(--priority-fill-3); --circle-border: var(--priority-3); }
    .pri-bg-4 { --circle-fill: var(--priority-fill-4); --circle-border: var(--priority-4); }
</style>
