<script lang="ts">
    import type { AppTask } from '../../lib/domain';
    import { persistedKeys, persistState } from '../../lib/persisted.svelte';
    import { ymd } from '../../lib/time';
    import WrapNextUpCard from './WrapNextUpCard.svelte';
    import WrapTomorrowPicker from './WrapTomorrowPicker.svelte';
    import {
        EMPTY_REFLECTION,
        ENERGY_OPTIONS,
        isValidWrapReflections,
        MOOD_OPTIONS,
        type WrapReflectionEntry,
        type WrapReflectionMap,
    } from './reflections';

    interface Props {
        wrapDate: Date;
        tomorrow: Date;
        tasks: readonly AppTask[];
        completedTasks: readonly AppTask[];
    }

    let { wrapDate, tomorrow, tasks, completedTasks }: Props = $props();

    let wrapYmd = $derived(ymd(wrapDate));
    let tomorrowYmd = $derived(ymd(tomorrow));

    let reflections = $state<WrapReflectionMap>({});

    persistState(
        persistedKeys.wrapReflections,
        () => reflections,
        (stored) => {
            reflections = stored;
        },
        { isValid: isValidWrapReflections },
    );

    let current = $derived(reflections[wrapYmd] ?? EMPTY_REFLECTION);

    function updateReflection(patch: Partial<WrapReflectionEntry>): void {
        const base = reflections[wrapYmd] ?? { wentWell: '', improve: '' };
        const next: WrapReflectionEntry = { ...base, ...patch };
        if (next.mood === undefined) delete next.mood;
        if (next.energy === undefined) delete next.energy;
        reflections = { ...reflections, [wrapYmd]: next };
    }

    function toggleMood(level: number): void {
        updateReflection({ mood: current.mood === level ? undefined : level });
    }

    function toggleEnergy(level: number): void {
        updateReflection({ energy: current.energy === level ? undefined : level });
    }
</script>

<div class="wrap-right-pane">
    <WrapNextUpCard {tasks} {completedTasks} {wrapYmd} {tomorrowYmd} />

    <WrapTomorrowPicker
        {tasks}
        tomorrowDate={tomorrow}
        {tomorrowYmd}
    />

    <section class="wrap-card" aria-label="Mood and energy check-in">
        <header class="wrap-card-hd">
            <h3 class="wrap-card-title">Check-in</h3>
            <span class="wrap-card-subtitle">mood & energy</span>
        </header>

        <div class="wrap-checkin-rows">
            <div class="wrap-checkin-row">
                <span class="wrap-checkin-label">Mood</span>
                <div class="wrap-segmented" role="group" aria-label="Mood">
                    {#each MOOD_OPTIONS as opt (opt.value)}
                        <button
                            type="button"
                            class="wrap-seg-btn"
                            class:is-selected={current.mood === opt.value}
                            aria-pressed={current.mood === opt.value}
                            onclick={() => toggleMood(opt.value)}
                        >
                            <span class="wrap-seg-num">{opt.value}</span>
                            <span>{opt.label}</span>
                        </button>
                    {/each}
                </div>
            </div>

            <div class="wrap-checkin-row">
                <span class="wrap-checkin-label">Energy</span>
                <div class="wrap-segmented" role="group" aria-label="Energy">
                    {#each ENERGY_OPTIONS as opt (opt.value)}
                        <button
                            type="button"
                            class="wrap-seg-btn"
                            class:is-selected={current.energy === opt.value}
                            aria-pressed={current.energy === opt.value}
                            onclick={() => toggleEnergy(opt.value)}
                        >
                            <span class="wrap-seg-num">{opt.value}</span>
                            <span>{opt.label}</span>
                        </button>
                    {/each}
                </div>
            </div>
        </div>
    </section>

    <section class="wrap-card" aria-label="Daily reflection">
        <header class="wrap-card-hd">
            <h3 class="wrap-card-title">Daily reflection</h3>
            <span class="wrap-card-subtitle">{wrapYmd}</span>
        </header>

        <label class="wrap-field">
            <span class="wrap-field-label">What went well today?</span>
            <textarea
                class="wrap-textarea"
                value={current.wentWell}
                oninput={(e) => updateReflection({ wentWell: e.currentTarget.value })}
                placeholder="Wins, steady focus moments, or things that felt good..."
            ></textarea>
        </label>

        <label class="wrap-field">
            <span class="wrap-field-label">How could productivity improve tomorrow?</span>
            <textarea
                class="wrap-textarea"
                value={current.improve}
                oninput={(e) => updateReflection({ improve: e.currentTarget.value })}
                placeholder="Friction to remove, better starting cue, or adjustments..."
            ></textarea>
        </label>
    </section>
</div>
