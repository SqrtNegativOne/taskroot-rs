<script lang="ts">
    let {
        priority,
        isDoneOrChecking,
        isActive,
        onclick,
        oncontextmenu,
        title = "Toggle Done",
        ariaLabel,
    }: {
        priority?: number | string;
        isDoneOrChecking: boolean;
        isActive?: boolean;
        onclick: (e: MouseEvent) => void;
        oncontextmenu?: (e: MouseEvent) => void;
        title?: string;
        ariaLabel?: string;
    } = $props();
</script>

<button
    type="button"
    class="task-circle {priority !== undefined ? `pri-bg-${priority.toString()}` : ''}"
    style="border: none; padding: 0; font: inherit; color: inherit; cursor: pointer;"
    {title}
    aria-label={ariaLabel ?? (priority !== undefined ? `Priority ${priority.toString()}` : 'Toggle Done')}
    {onclick}
    {oncontextmenu}
>
    {#if isActive}
        <svg
            class="task-circle-play"
            viewBox="0 0 24 24"
            fill="currentColor"
            style="width: 12px; height: 12px; color: var(--bg);"
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
