<script lang="ts">
    import CalendarManager from '../../components/CalendarManager.svelte';
    import AccountProfiles from '../../components/accounts/AccountProfiles.svelte';
    import type { AccountRegistry } from '../../lib/bindings/AccountRegistry.generated';
    import type { AppCalendar } from '../../lib/domain';
    import { activeCalendars } from '../../lib/queries';
    import { useAutoQuery } from '../../lib/safeInvoke.svelte';

    const accounts = useAutoQuery<AccountRegistry>('get_accounts', () => ({}));
    const calendars = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);
</script>

<AccountProfiles />

{#if accounts.data?.activeId}
    <section class="settings-section">
        <h2>Lists</h2>
        <CalendarManager calendars={calendars.data ?? []} />
    </section>
{/if}

<style>
    .settings-section { margin: 30px 0 40px; }
    h2 { font-size: 16px; color: var(--fg-dim); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 15px; border-bottom: 1px solid var(--border); padding-bottom: 5px; }
</style>
