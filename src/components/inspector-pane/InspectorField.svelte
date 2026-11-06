<script lang="ts">
    import type { Snippet } from 'svelte';
    import Icon from '../Icon.svelte';

    interface Props {
        icon: string;
        label: string;
        disabled?: boolean;
        multiline?: boolean;
        children: Snippet;
    }

    let { icon, label, disabled = false, multiline = false, children }: Props = $props();
</script>

<div
    class="inspector-field"
    class:is-disabled={disabled}
    class:is-multiline={multiline}
    data-field={label}
>
    <span class="inspector-field-icon" aria-hidden="true">
        <Icon name={icon} size={18} />
    </span>
    <div class="inspector-field-value">
        {@render children()}
    </div>
</div>
