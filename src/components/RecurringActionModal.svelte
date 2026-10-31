<script lang="ts">
    import { Dialog } from 'bits-ui';
    import type { EventScope } from '../lib/domain';

    interface Props {
        isOpen: boolean;
        actionType: "edit" | "delete" | "move";
        /** Which choices to offer; a move cannot re-anchor a series from one occurrence. */
        modes?: EventScope[];
        onConfirm: (mode: EventScope) => void;
        onCancel: () => void;
    }

    let {
        isOpen,
        actionType,
        modes = ["this", "following", "all"],
        onConfirm,
        onCancel,
    }: Props = $props();

    let selectedMode = $state<EventScope>("this");

    // Reset selection when modal opens
    $effect(() => {
        if (isOpen) {
            selectedMode = modes.includes("this") ? "this" : modes[0] ?? "this";
        }
    });

    let actionText = $derived(actionType === "delete" ? "Delete" : actionType === "move" ? "Move" : "Edit");

    function handleConfirm() {
        onConfirm(selectedMode);
    }

    function handleOpenChange(open: boolean) {
        if (!open) onCancel();
    }
</script>

<Dialog.Root open={isOpen} onOpenChange={handleOpenChange}>
    <Dialog.Portal>
        <Dialog.Overlay
            style="position: fixed; inset: 0; background-color: rgba(0, 0, 0, 0.5); z-index: 99999;"
        />
        <Dialog.Content
            style="position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); background: var(--bg-panel, #1e1e1e); padding: 24px; border-radius: 8px; width: 320px; box-shadow: 0 4px 12px rgba(0,0,0,0.3); color: var(--fg); z-index: 100000;"
        >
            <Dialog.Title level={3} style="margin: 0 0 16px 0; font-size: 16px; font-weight: 600;">
                {actionText} recurring event
            </Dialog.Title>

            <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 24px;">
                {#if modes.includes("this")}
                    <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 14px;">
                        <input type="radio" name="recurring_mode" value="this" bind:group={selectedMode} />
                        This event
                    </label>
                {/if}
                {#if modes.includes("following")}
                    <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 14px;">
                        <input type="radio" name="recurring_mode" value="following" bind:group={selectedMode} />
                        This and following events
                    </label>
                {/if}
                {#if modes.includes("all")}
                    <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 14px;">
                        <input type="radio" name="recurring_mode" value="all" bind:group={selectedMode} />
                        All events
                    </label>
                {/if}
            </div>

            <div style="display: flex; justify-content: flex-end; gap: 12px;">
                <button 
                    onclick={onCancel}
                    style="background: transparent; border: none; color: var(--fg-muted); cursor: pointer; padding: 6px 12px; font-size: 14px;"
                >
                    Cancel
                </button>
                <button 
                    onclick={handleConfirm}
                    style="background: var(--accent, #3b82f6); border: none; color: #fff; cursor: pointer; padding: 6px 16px; border-radius: 4px; font-weight: 500; font-size: 14px;"
                >
                    OK
                </button>
            </div>
        </Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>
