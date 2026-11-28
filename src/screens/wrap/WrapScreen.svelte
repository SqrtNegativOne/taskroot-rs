<script lang="ts">
    import './wrap.css';
    import { goto } from '$app/navigation';
    import { resolve } from '$app/paths';
    import { Routes } from '../../lib/routes';
    import { useNow } from '../../lib/useNow.svelte';
    import { isPostMidnightWrap, isWrapWindowOpen, tomorrowOf, wrapTargetDate } from './wrapWindow';
    import WrapDayView from './WrapDayView.svelte';

    const now = useNow(60_000);
    let nowDate = $derived(new Date(now.ms));
    let isOpen = $derived(isWrapWindowOpen(nowDate));
    let isYesterday = $derived(isPostMidnightWrap(nowDate));
    let wrapDate = $derived(wrapTargetDate(nowDate));
    let tomorrow = $derived(tomorrowOf(wrapDate));

    function backToPlan(): void {
        void goto(resolve(Routes.HOME));
    }
</script>

{#if !isOpen}
    <main class="wrap-closed-shell">
        <section class="wrap-closed-card">
            <span class="wrap-closed-eyebrow">Evening wrap · 10:00 PM – 3:00 AM</span>
            <h1>Wrap opens at 10:00 PM</h1>
            <p>Check back tonight between 10:00 PM and 3:00 AM to review your day.</p>
            <button type="button" class="wrap-primary-btn" onclick={backToPlan}>
                Back to Plan
            </button>
        </section>
    </main>
{:else}
    <WrapDayView {wrapDate} {tomorrow} {isYesterday} />
{/if}
