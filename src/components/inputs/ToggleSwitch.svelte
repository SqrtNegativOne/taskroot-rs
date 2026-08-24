<script lang="ts">
    interface Props {
        checked: boolean;
        onchange: (checked: boolean) => void;
        ariaLabel?: string;
        class?: string;
        style?: string;
    }

    let { checked, onchange, ariaLabel = "Toggle", class: className = '', style = '' }: Props = $props();
</script>

<button
    type="button"
    aria-label={ariaLabel}
    style={`display: flex; gap: 12px; align-items: center; color: var(--fg); cursor: pointer; background: none; border: none; padding: 0; ${style}`}
    onclick={() => onchange(!checked)}
    class={className}
    data-cuelume-toggle
>
    <div class={`toggle-switch ${checked ? "is-on" : ""}`}>
        <div class="toggle-switch-thumb"></div>
    </div>
</button>
