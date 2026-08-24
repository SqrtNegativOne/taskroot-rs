<script lang="ts">
    interface Props {
        title: string;
    }

    let { title }: Props = $props();
</script>

<div class="coming-soon">
    <h2>{title}</h2>
    <p>This screen has not been implemented yet.</p>
</div>

<style>
    .coming-soon {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 100%;
        color: var(--fg-dim);
        text-align: center;
    }

    .coming-soon h2 {
        margin: 0 0 8px;
        font-weight: 400;
        letter-spacing: 0.05em;
        color: var(--fg-muted);
    }

    .coming-soon p {
        margin: 0;
        font-size: 0.9em;
    }
</style>
