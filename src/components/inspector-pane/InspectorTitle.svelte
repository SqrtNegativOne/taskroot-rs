<script lang="ts">
    import { onDestroy, untrack } from 'svelte';
    import { createDebouncer } from '../../lib/asyncState.svelte';
    import type { AppCalendar } from '../../lib/domain';
    import {
        hasTrailingIncompleteSigil,
        parseTaskTitleQuickEdit,
        type TaskTitleQuickEdit,
    } from './quickEdit';

    export interface InspectorTitleQuickEdit {
        calendars: readonly AppCalendar[];
        onCommit: (edit: TaskTitleQuickEdit) => void;
        onError?: (message: string) => void;
    }

    interface Props {
        value: string;
        onchange: (value: string) => void;
        disabled?: boolean;
        quickEdit?: InspectorTitleQuickEdit;
    }

    let { value, onchange, disabled = false, quickEdit }: Props = $props();

    let localValue = $state(untrack(() => value));
    let lastPropValue = untrack(() => value);
    let lastSavedValue = untrack(() => value);
    let focused = $state(false);
    let dirty = $state(false);
    let ignoreQuickEdit = $state(false);
    let referenceDate = $state(new Date());

    const writer = createDebouncer<string>((next) => {
        dirty = false;
        lastSavedValue = next;
        onchange(next);
    }, 400);

    $effect(() => {
        const incoming = value;
        if (incoming !== lastPropValue) {
            lastPropValue = incoming;
            lastSavedValue = incoming;
            if (!focused) {
                localValue = incoming;
                dirty = false;
            }
        }
    });

    let parsedEdit = $derived(
        quickEdit !== undefined && dirty && !ignoreQuickEdit && !disabled
            ? parseTaskTitleQuickEdit(localValue, value, referenceDate, quickEdit.calendars)
            : null,
    );

    let showPreview = $derived(
        focused && parsedEdit !== null && (parsedEdit.hasProperties || parsedEdit.listError !== null),
    );

    function autoGrow(node: HTMLTextAreaElement, _value: string) {
        function resize(): void {
            node.style.height = 'auto';
            node.style.height = `${node.scrollHeight}px`;
        }
        resize();
        return { update: resize };
    }

    function handleInput() {
        dirty = true;
        if (quickEdit !== undefined && !ignoreQuickEdit) {
            const candidate = parseTaskTitleQuickEdit(
                localValue,
                value,
                referenceDate,
                quickEdit.calendars,
            );
            if (
                candidate.hasProperties ||
                candidate.listError !== null ||
                hasTrailingIncompleteSigil(localValue)
            ) {
                writer.cancel();
                return;
            }
        }
        writer.schedule(localValue);
    }

    function commitPending(): void {
        if (quickEdit !== undefined && dirty && !ignoreQuickEdit && !disabled) {
            const edit = parseTaskTitleQuickEdit(
                localValue,
                value,
                referenceDate,
                quickEdit.calendars,
            );
            if (edit.listError !== null) {
                writer.cancel();
                quickEdit.onError?.(edit.listError);
                localValue = value;
                dirty = false;
                return;
            }
            if (edit.hasProperties) {
                writer.cancel();
                quickEdit.onCommit(edit);
                localValue = edit.title;
                lastSavedValue = edit.title;
                dirty = false;
                return;
            }
        }

        if (dirty) {
            writer.cancel();
            dirty = false;
            if (localValue !== lastSavedValue) {
                lastSavedValue = localValue;
                onchange(localValue);
            }
            return;
        }
        writer.flush();
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key !== 'Enter') return;
        event.preventDefault();
        if (parsedEdit?.listError !== null && parsedEdit?.listError !== undefined) {
            return;
        }
        (event.currentTarget as HTMLTextAreaElement).blur();
    }

    function handleFocus() {
        focused = true;
        referenceDate = new Date();
    }

    function handleBlur() {
        focused = false;
        commitPending();
        ignoreQuickEdit = false;
    }

    function keepAsText() {
        ignoreQuickEdit = true;
        writer.schedule(localValue);
    }

    function preventBlur(event: Event) {
        event.preventDefault();
    }

    onDestroy(() => commitPending());
</script>

<textarea
    bind:value={localValue}
    use:autoGrow={localValue}
    {disabled}
    rows="1"
    class="inspector-title-input"
    placeholder="Untitled"
    aria-label="Title"
    spellcheck="false"
    oninput={handleInput}
    onkeydown={handleKeydown}
    onfocus={handleFocus}
    onblur={handleBlur}
></textarea>

{#if showPreview && parsedEdit}
    <div
        class="inspector-quick-preview"
        class:is-error={parsedEdit.listError !== null}
        role="status"
        aria-live="polite"
    >
        {#if parsedEdit.listError}
            <span class="inspector-quick-error">{parsedEdit.listError}</span>
        {:else}
            <span class="inspector-quick-parts">{parsedEdit.previewParts.join(' · ')}</span>
        {/if}
        <button
            type="button"
            class="inspector-quick-dismiss"
            onpointerdown={preventBlur}
            onmousedown={preventBlur}
            onclick={keepAsText}
        >
            Keep text
        </button>
    </div>
{/if}
