<script lang="ts">
    let { message, closeToast }: { message: string; closeToast?: () => void } = $props();

    let copied = $state(false);

    async function copy(): Promise<void> {
        try {
            await navigator.clipboard.writeText(message);
            copied = true;
            setTimeout(() => {
                copied = false;
            }, 150);
        } catch (error) {
            console.error('Failed to copy notification:', error);
        }
    }
</script>

<div class="copyable-toast" role="alert">
    <button
        type="button"
        class="copyable-toast-message"
        class:copied
        title="Click to copy"
        onclick={copy}
    >
        {message}
    </button>
    <button type="button" class="copyable-toast-dismiss" title="Dismiss" onclick={() => closeToast?.()}>
        &times;
    </button>
</div>

<style>
    .copyable-toast {
        display: flex;
        align-items: center;
        gap: 12px;
        width: 100%;
        padding: 12px 16px;
        color: var(--danger-fg);
        font-family: inherit;
        font-size: 0.95rem;
        font-weight: 500;
        text-align: left;
        background: var(--danger-bg);
        border: 1px solid var(--danger-border);
        border-radius: 0;
        box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.3), 0 8px 10px -6px rgba(0, 0, 0, 0.2);
        backdrop-filter: blur(8px);
        -webkit-backdrop-filter: blur(8px);
        max-width: 350px;
    }

    .copyable-toast-message {
        flex: 1;
        padding: 0;
        color: inherit;
        font: inherit;
        text-align: left;
        word-break: break-word;
        background: transparent;
        border: none;
        border-radius: 0;
        cursor: pointer;
        transition: background 0.15s ease;
    }

    .copyable-toast-message.copied {
        background: color-mix(in oklch, var(--danger-fg) 20%, transparent);
    }

    .copyable-toast-dismiss {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 4px;
        color: currentColor;
        font-size: 1.6rem;
        line-height: 1;
        background: transparent;
        border: none;
        border-radius: 0;
        opacity: 0.7;
        cursor: pointer;
    }
</style>
