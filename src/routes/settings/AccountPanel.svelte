<script lang="ts">
    import { onMount } from 'svelte';
    import CalendarManager from '../../components/CalendarManager.svelte';
    import type { Account } from '../../lib/bindings/Account.generated';
    import type { AppCalendar } from '../../lib/domain';
    import { activeCalendars } from '../../lib/queries';
    import { describeAppError, safeInvoke, useAutoQuery } from '../../lib/safeInvoke.svelte';

    interface Props {
        onSignedOut: () => void;
    }

    let { onSignedOut }: Props = $props();

    let account = $state<Account | null>(null);
    let accountError = $state<string | null>(null);
    let isSigningOut = $state(false);

    const calendarsQuery = useAutoQuery<AppCalendar[]>('get_active_calendars', activeCalendars);

    onMount(() => {
        void loadAccount();
    });

    async function loadAccount(): Promise<void> {
        const result = await safeInvoke<Account | null>('get_account');
        result.match(
            (loaded) => (account = loaded),
            (error) => (accountError = describeAppError(error)),
        );
    }

    async function signOut(): Promise<void> {
        if (!account || isSigningOut) return;
        const question = `Sign out of ${account.serverUrl}? You will need the server password to sign back in.`;
        if (!confirm(question)) return;

        isSigningOut = true;
        accountError = null;
        const result = await safeInvoke('disconnect_account');
        isSigningOut = false;

        if (result.isErr()) {
            accountError = `Could not sign out: ${describeAppError(result.error)}`;
            return;
        }
        account = null;
        onSignedOut();
    }
</script>

{#if accountError}
    <div class="account-error" role="alert">{accountError}</div>
{/if}

{#if account}
    <section class="account-card">
        <p class="account-who">
            <span class="account-user">{account.username}</span>
            <span class="account-server">{account.serverUrl}</span>
        </p>
        <button class="sign-out" onclick={() => void signOut()} disabled={isSigningOut}>
            {isSigningOut ? 'Signing out…' : 'Sign out'}
        </button>
    </section>

    <section class="settings-section">
        <h2>Lists</h2>
        <CalendarManager calendars={calendarsQuery.data ?? []} />
    </section>
{:else}
    <p class="account-empty">No account connected.</p>
{/if}

<style>
    .account-error {
        margin-bottom: 20px;
        padding: 10px 14px;
        border: 1px solid var(--tag-red);
        border-radius: 4px;
        color: var(--tag-red);
        font-size: 13px;
    }

    .account-card {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 20px;
        margin-bottom: 30px;
        padding: 14px 18px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 6px;
    }

    .account-who {
        margin: 0;
        font-size: 14px;
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        align-items: baseline;
    }

    .account-user {
        font-weight: 600;
    }

    .account-server {
        color: var(--fg-dim);
    }

    .sign-out {
        flex-shrink: 0;
        padding: 7px 12px;
        font-size: 13px;
        color: var(--tag-red);
        background: transparent;
        border: 1px solid var(--tag-red);
        border-radius: 4px;
        cursor: pointer;
    }

    .sign-out:disabled {
        opacity: 0.6;
        cursor: default;
    }

    .account-empty {
        color: var(--fg-dim);
        font-size: 14px;
    }

    .settings-section {
        margin-bottom: 40px;
    }

    .settings-section h2 {
        font-size: 16px;
        color: var(--fg-dim);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-bottom: 15px;
        border-bottom: 1px solid var(--border);
        padding-bottom: 5px;
    }
</style>
