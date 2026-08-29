import { useNow } from './src/lib/useNow.svelte';
import { effect } from 'svelte/reactivity';

const now = useNow();
console.log(now.ms);
