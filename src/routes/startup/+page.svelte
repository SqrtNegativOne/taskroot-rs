<script lang="ts">
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Routes } from '$lib/routes';

    function startDay(): void {
        void goto(resolve(Routes.HOME));
    }
</script>

<div class="startup-shell">
    <section class="startup-card">
        <h1>Welcome back</h1>
        <p>Take a moment to settle in. Your day is waiting when you are.</p>
        <button type="button" onclick={startDay}>Go to plan</button>
    </section>
</div>

<style>
    .startup-shell {
        display: flex;
        align-items: center;
        justify-content: center;
        min-height: 100%;
        padding: 40px 20px;
        background: var(--bg);
        color: var(--fg);
    }

    .startup-card {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 12px;
        width: min(420px, 100%);
        padding: 32px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 8px;
        box-shadow: var(--shadow-btn);
    }

    h1 {
        margin: 0;
        font-size: 1.6em;
        font-weight: 400;
        letter-spacing: 0.02em;
    }

    p {
        margin: 0;
        font-size: 0.9em;
        line-height: 1.6;
        color: var(--fg-muted);
    }

    button {
        margin-top: 10px;
        padding: 8px 16px;
        font-size: 0.9em;
        color: var(--bg);
        background: var(--accent);
        border: 1px solid transparent;
        border-radius: 4px;
        cursor: pointer;
    }
</style>
