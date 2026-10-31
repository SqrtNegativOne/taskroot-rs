import { toast } from 'svelte-sonner';
import CopyableToast from '../components/CopyableToast.svelte';

export function notifyError(message: string): void {
    toast.custom(CopyableToast, {
        componentProps: { message },
        duration: Infinity,
    });
}
