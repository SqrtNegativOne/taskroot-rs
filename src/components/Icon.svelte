<script lang="ts">
    interface Props {
        name: string;
        size?: number | string;
        class?: string;
        style?: string;
    }

    let {
        name,
        size = 'inherit',
        class: className = '',
        style = ''
    }: Props = $props();

    let computedStyle = $derived(`font-size: ${typeof size === 'number' ? size + 'px' : size}; ${style}`);
</script>

<span
    class={`material-symbols-outlined ${className}`}
    style={computedStyle}
>
    {name}
</span>
