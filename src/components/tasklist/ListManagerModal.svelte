<script lang="ts">
    import { Dialog } from 'bits-ui';
    import type { AppCalendar } from '../../lib/domain';
    import CalendarManager from '../CalendarManager.svelte';

    interface Props {
        isOpen: boolean;
        calendars: AppCalendar[];
        onClose: () => void;
    }

    let { isOpen, calendars, onClose }: Props = $props();

    function handleOpenChange(open: boolean): void {
        if (!open) onClose();
    }
</script>

<Dialog.Root open={isOpen} onOpenChange={handleOpenChange}>
    <Dialog.Portal>
        <Dialog.Overlay
            style="position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 99999;"
        />
        <Dialog.Content
            style="position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); background: var(--bg-panel, #1e1e1e); padding: 20px 24px; border-radius: 8px; width: 460px; max-height: 80vh; display: flex; flex-direction: column; box-shadow: 0 4px 12px rgba(0,0,0,0.3); color: var(--fg); z-index: 100000;"
        >
            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px;">
                <Dialog.Title level={3} style="margin: 0; font-size: 16px; font-weight: 600;">Lists</Dialog.Title>
                <button class="icon-btn" onclick={onClose} aria-label="Close">✕</button>
            </div>

            <div style="overflow-y: auto;">
                <CalendarManager {calendars} />
            </div>
        </Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>
