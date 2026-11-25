<script lang="ts">
    import AccountBoundary from '../components/accounts/AccountBoundary.svelte';
    let { preserve = false }: { preserve?: boolean } = $props();
</script>

<AccountBoundary {preserve}>
    <label>Draft<input /></label>
</AccountBoundary>
