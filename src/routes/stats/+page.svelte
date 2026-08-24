<script lang="ts">
    import ComingSoon from '../../components/ComingSoon.svelte';
    import { Routes } from '$lib/routes';

    const routeName = Routes.STATS.substring(1);
    const title = `${routeName.charAt(0).toUpperCase()}${routeName.substring(1)}`;
</script>

<ComingSoon {title} />
