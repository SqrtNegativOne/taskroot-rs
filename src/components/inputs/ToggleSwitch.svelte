<script lang="ts">
    interface Props {
        checked: boolean;
        onchange: (checked: boolean) => void;
        ariaLabel?: string;
        class?: string;
        style?: string;
        disabled?: boolean;
    }

    let {
        checked,
        onchange,
        ariaLabel = "Toggle",
        class: className = '',
        style = '',
        disabled = false,
    }: Props = $props();
</script>

<button
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={ariaLabel}
    {disabled}
    style={`display: flex; gap: 12px; align-items: center; color: var(--fg); cursor: ${disabled ? 'not-allowed' : 'pointer'}; background: none; border: none; padding: 0; opacity: ${disabled ? 0.5 : 1}; ${style}`}
    onclick={() => onchange(!checked)}
    class={className}
    data-cuelume-toggle
>
    <span class={`toggle-switch ${checked ? "is-on" : ""}`}>
        <span class="toggle-switch-thumb"></span>
    </span>
</button>

<style>
    .toggle-switch {
        position: relative;
        width: 36px;
        height: 20px;
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: 10px;
        transition: background 0.2s, border-color 0.2s;
    }
    .toggle-switch.is-on {
        background: var(--accent);
        border-color: var(--accent);
    }
    .toggle-switch-thumb {
        position: absolute;
        top: 1px;
        left: 1px;
        width: 16px;
        height: 16px;
        background: white;
        border-radius: 50%;
        transition: transform 0.2s;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    }
    .toggle-switch.is-on .toggle-switch-thumb {
        transform: translateX(16px);
    }
</style>
