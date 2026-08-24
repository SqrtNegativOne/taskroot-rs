<script module lang="ts">
    export type RecurringMode = "instance" | "following" | "all";
</script>

<script lang="ts">
    interface Props {
        isOpen: boolean;
        actionType: "edit" | "delete";
        onConfirm: (mode: RecurringMode) => void;
        onCancel: () => void;
    }

    let { isOpen, actionType, onConfirm, onCancel }: Props = $props();

    let selectedMode = $state<RecurringMode>("instance");

    // Reset selection when modal opens
    $effect(() => {
        if (isOpen) {
            selectedMode = "instance";
        }
    });

    let actionText = $derived(actionType === "edit" ? "Edit" : "Delete");

    function handleConfirm() {
        onConfirm(selectedMode);
    }
</script>

{#if isOpen}
    <div style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background-color: rgba(0, 0, 0, 0.5); display: flex; align-items: center; justify-content: center; z-index: 99999;">
        <div style="background: var(--bg-panel, #1e1e1e); padding: 24px; border-radius: 8px; width: 320px; box-shadow: 0 4px 12px rgba(0,0,0,0.3); color: var(--fg-primary, #fff);">
            <h3 style="margin: 0 0 16px 0; font-size: 16px; font-weight: 600;">{actionText} recurring event</h3>
            
            <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 24px;">
                <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 14px;">
                    <input type="radio" name="recurring_mode" value="instance" bind:group={selectedMode} />
                    This event
                </label>
                <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 14px;">
                    <input type="radio" name="recurring_mode" value="following" bind:group={selectedMode} />
                    This and following events
                </label>
                <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 14px;">
                    <input type="radio" name="recurring_mode" value="all" bind:group={selectedMode} />
                    All events
                </label>
            </div>

            <div style="display: flex; justify-content: flex-end; gap: 12px;">
                <button 
                    onclick={onCancel}
                    style="background: transparent; border: none; color: var(--fg-secondary, #aaa); cursor: pointer; padding: 6px 12px; font-size: 14px;"
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
        </div>
    </div>
{/if}
