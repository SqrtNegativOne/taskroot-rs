<script lang="ts">
    import { page } from '$app/state';

    const isChunkFailure = $derived(
        page.error?.message?.includes('Failed to fetch dynamically imported module') ||
        page.error?.message?.includes('error loading dynamically imported module') ||
        page.error?.message?.includes('Importing a module script failed')
    );

    function reload() {
        window.location.reload();
    }

    function goHome() {
        window.location.href = '/';
    }
</script>

<div class="error-screen">
    <div class="error-card">
        <div class="status-badge">{page.status}</div>
        <h1 class="error-title">
            {#if isChunkFailure}
                Connection interrupted
            {:else}
                {page.error?.message ?? 'Internal Error'}
            {/if}
        </h1>
        <p class="error-description">
            {#if isChunkFailure}
                A module failed to load. This typically happens when your computer resumes from sleep or when the dev server restarts.
            {:else}
                An unexpected error occurred while loading this view.
            {/if}
        </p>
        <div class="actions">
            <button class="btn btn-primary" onclick={reload}>
                <span class="icon">refresh</span>
                Reload
            </button>
            <button class="btn btn-secondary" onclick={goHome}>
                <span class="icon">home</span>
                Home
            </button>
        </div>
    </div>
</div>

<style>
    .error-screen {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100vw;
        height: 100vh;
        background: var(--bg);
        color: var(--fg);
        font-family: var(--mono);
        padding: 24px;
        box-sizing: border-box;
    }

    .error-card {
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
        max-width: 440px;
        padding: 32px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 8px;
    }

    .status-badge {
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.08em;
        padding: 2px 8px;
        border-radius: 4px;
        background: var(--bg-highlight);
        color: var(--accent);
        border: 1px solid var(--border-soft);
        margin-bottom: 16px;
    }

    .error-title {
        font-size: 16px;
        font-weight: 600;
        margin: 0 0 8px;
        color: var(--fg);
    }

    .error-description {
        font-size: 12px;
        color: var(--fg-muted);
        line-height: 1.5;
        margin: 0 0 24px;
    }

    .actions {
        display: flex;
        gap: 12px;
    }

    .btn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 8px 16px;
        font-family: var(--mono);
        font-size: 12px;
        border-radius: 4px;
        cursor: pointer;
        transition: background 0.15s ease, border-color 0.15s ease;
    }

    .btn-primary {
        background: var(--accent);
        color: var(--bg);
        border: 1px solid var(--accent);
        font-weight: 600;
    }

    .btn-primary:hover {
        opacity: 0.9;
    }

    .btn-secondary {
        background: transparent;
        color: var(--fg-muted);
        border: 1px solid var(--border);
    }

    .btn-secondary:hover {
        background: var(--bg-surface-hover);
        color: var(--fg);
    }

    .icon {
        font-family: 'Material Symbols Outlined';
        font-size: 16px;
        line-height: 1;
    }
</style>
