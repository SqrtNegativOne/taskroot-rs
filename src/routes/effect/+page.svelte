<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { listen, type UnlistenFn } from '@tauri-apps/api/event';
    import { safeInvoke } from '$lib/safeInvoke.svelte';
    import { SCREEN_EFFECT } from '$lib/events';

    const SWEEP_COLOR = '#00ffcc';
    const FLASH_COLOR = '#ffffff';
    const FRAME_MS = 16;
    const FLASH_MS = 80;
    const FADE_MS = 30;

    let canvas = $state<HTMLCanvasElement | null>(null);
    let ctx: CanvasRenderingContext2D | null = null;
    let opacity = $state(1);

    // Invalidates any in-flight run when a new effect is triggered or the route
    // is destroyed, so two overlapping animations cannot fight over the canvas.
    let animationToken = 0;

    /**
     * A point on the sweeping path: from bottom-center, up the side, and across
     * the top to the center. Mirrors the reference `get_points` geometry.
     */
    function points(t: number, isLeft: boolean, width: number, height: number): number[] {
        const half = width / 2;
        const total = half + height + half;
        let remaining = t * total;
        const sign = isLeft ? -1 : 1;
        const out: number[] = [half, height];

        if (remaining <= half) {
            out.push(half + sign * remaining, height);
            return out;
        }
        out.push(half + sign * half, height);
        remaining -= half;

        if (remaining <= height) {
            out.push(half + sign * half, height - remaining);
            return out;
        }
        out.push(half + sign * half, 0);
        remaining -= height;

        if (remaining <= half) {
            out.push(half + sign * half - sign * remaining, 0);
            return out;
        }
        out.push(half, 0);
        return out;
    }

    function draw(progress: number, color: string, lineWidth: number): void {
        if (!ctx || !canvas) return;
        const { width, height } = canvas;
        ctx.clearRect(0, 0, width, height);
        ctx.strokeStyle = color;
        ctx.lineWidth = lineWidth;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';

        for (const isLeft of [true, false]) {
            const pts = points(progress, isLeft, width, height);
            ctx.beginPath();
            ctx.moveTo(pts[0], pts[1]);
            for (let i = 2; i < pts.length; i += 2) {
                ctx.lineTo(pts[i], pts[i + 1]);
            }
            ctx.stroke();
        }
    }

    function delay(ms: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }

    async function runEffect(token: number): Promise<void> {
        if (!canvas) return;
        opacity = 1;
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;

        for (let progress = 0; progress <= 1.0001; progress += 0.02) {
            if (token !== animationToken) return;
            draw(progress, SWEEP_COLOR, 2);
            await delay(FRAME_MS);
        }

        for (let flash = 0; flash < 6; flash += 1) {
            if (token !== animationToken) return;
            const bright = flash % 2 === 0;
            draw(1, bright ? FLASH_COLOR : SWEEP_COLOR, bright ? 3 : 2);
            await delay(FLASH_MS);
        }

        for (let alpha = 1; alpha > 0; alpha -= 0.05) {
            if (token !== animationToken) return;
            opacity = alpha;
            await delay(FADE_MS);
        }

        if (token !== animationToken) return;
        opacity = 1;
        await safeInvoke('hide_screen_effect');
    }

    let unlisten: UnlistenFn | undefined;

    onMount(() => {
        if (!canvas) return;
        ctx = canvas.getContext('2d');
        void listen(SCREEN_EFFECT, () => {
            animationToken += 1;
            void runEffect(animationToken);
        }).then((un) => {
            unlisten = un;
        });
    });

    onDestroy(() => {
        animationToken += 1;
        unlisten?.();
    });
</script>

<canvas bind:this={canvas} style="opacity: {opacity}"></canvas>

<style>
    :global(html),
    :global(body) {
        background: transparent !important;
        margin: 0;
        padding: 0;
        overflow: hidden;
    }

    canvas {
        display: block;
        width: 100vw;
        height: 100vh;
        transition: opacity 30ms linear;
    }
</style>
