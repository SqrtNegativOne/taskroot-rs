<script lang="ts">
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Routes } from '$lib/routes';
    import AccountProfiles from '../../components/accounts/AccountProfiles.svelte';
</script>

<div class="connect-shell">
    <main class="connect-card">
        <h1>Connect CalDAV</h1>
        <AccountProfiles showAddInitially onActivated={() => goto(resolve(Routes.HOME))} />
    </main>
</div>

<style>
    .connect-shell { display: flex; align-items: center; justify-content: center; min-height: 100%; padding: 40px 20px; background: var(--bg); color: var(--fg); }
    .connect-card { width: min(640px, 100%); padding: 28px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; }
    h1 { margin: 0 0 20px; font-size: 1.4em; }
</style>
