<script lang="ts">
    import { onMount } from 'svelte';
    import './inspector.css';
    import type { AppTask, AppEvent, AppTaskStatus } from '../../lib/domain';

    let {
        inspectorState,
        onClose,
        tasks,
        events,
        updateTask,
        updateEvent,
        deleteTask,
        deleteEvent
    }: {
        inspectorState?: { type: 'task' | 'event'; id: string };
        onClose: () => void;
        tasks: AppTask[];
        events: AppEvent[];
        updateTask: (id: string, t: (t: AppTask) => AppTask) => void;
        updateEvent: (id: string, e: (e: AppEvent) => AppEvent) => void;
        deleteTask: (id: string) => void;
        deleteEvent: (id: string) => void;
    } = $props();

    let paneRef = $state<HTMLElement | null>(null);

    let currentItem = $derived.by(() => {
        if (!inspectorState) return null;
        if (inspectorState.type === 'task') return tasks.find(t => t.id === inspectorState.id);
        return events.find(e => e.id === inspectorState.id);
    });

    let isCurrentTask = $derived(inspectorState?.type === 'task');

    onMount(() => {
        function handleClickOutside(e: PointerEvent) {
            if (inspectorState && paneRef && e.target instanceof Node && !paneRef.contains(e.target)) {
                onClose();
            }
        }
        document.addEventListener('pointerdown', handleClickOutside);
        return () => { document.removeEventListener('pointerdown', handleClickOutside); };
    });

    function getFormattedDate(iso?: string) {
        if (!iso) return '';
        return iso.split('T')[0];
    }
    
    function getFormattedTime(iso?: string) {
        if (!iso?.includes('T')) return '';
        const t = iso.split('T')[1];
        return t.substring(0, 5); // HH:MM
    }
    
    function updateEventDate(ev: AppEvent, field: 'startTime' | 'endTime', dateStr: string) {
        if (!ev[field]) return;
        const timeStr = ev[field].split('T')[1] ?? '00:00:00';
        updateEvent(ev.id, e => ({ ...e, [field]: `${dateStr}T${timeStr}` }));
    }
    
    function updateEventTime(ev: AppEvent, field: 'startTime' | 'endTime', timeStr: string) {
        if (!ev[field]) return;
        const dateStr = ev[field].split('T')[0];
        updateEvent(ev.id, e => ({ ...e, [field]: `${dateStr}T${timeStr}:00` }));
    }
</script>

<div
    bind:this={paneRef}
    class="inspector-pane"
    class:is-open={!!currentItem}
>
    {#if currentItem}
        <header class="inspector-hd">
            <input 
                class="inspector-title-input" 
                value={currentItem.title}
                onchange={(e) => {
                    const val = e.currentTarget.value;
                    if (isCurrentTask) updateTask(currentItem.id, t => ({...t, title: val}));
                    else updateEvent(currentItem.id, ev => ({...ev, title: val}));
                }}
            />
            <button class="icon-btn" onclick={onClose}>✕</button>
        </header>

        <div class="inspector-body">
            <div class="inspector-field">
                <textarea 
                    id="desc"
                    class="inspector-desc-input"
                    placeholder="Description / Notes"
                    value={isCurrentTask ? ((currentItem as AppTask).notes ?? '') : ((currentItem as AppEvent).description ?? '')}
                    onchange={(e) => {
                        const val = e.currentTarget.value;
                        if (isCurrentTask) updateTask(currentItem.id, t => ({...t, notes: val}));
                        else updateEvent(currentItem.id, ev => ({...ev, description: val}));
                    }}
                ></textarea>
            </div>
            
            {#if isCurrentTask}
                {@const task = currentItem as AppTask}
                <div class="inspector-row">
                    <div class="inspector-field">
                        <label for={`status-${task.id}`}>Status</label>
                        <select 
                            id={`status-${task.id}`}
                            value={task.status ?? 'todo'}
                            onchange={(e) => {
                                const val = e.currentTarget.value as AppTaskStatus;
                                updateTask(task.id, t => ({...t, status: val}));
                            }}
                        >
                            <option value="todo">Todo</option>
                            <option value="doing">Doing</option>
                            <option value="next-up">Next Up</option>
                            <option value="done">Done</option>
                        </select>
                    </div>
                    <div class="inspector-field">
                        <label for={`priority-${task.id}`}>Priority</label>
                        <input
                            id={`priority-${task.id}`}
                            type="number"
                            min="0"
                            max="4"
                            value={task.priority ?? 2}
                            onchange={(e) => {
                                const val = parseInt(e.currentTarget.value) || 0;
                                const clamped = Math.max(0, Math.min(4, val));
                                if (clamped === 0 || clamped === 1 || clamped === 2 || clamped === 3 || clamped === 4) {
                                    updateTask(task.id, t => ({...t, priority: clamped}));
                                }
                            }}
                        />
                    </div>
                </div>
                
                <div class="inspector-field">
                    <label for={`due-${task.id}`}>Due Date</label>
                    <input
                        id={`due-${task.id}`}
                        class="inspector-date-input"
                        type="date"
                        value={getFormattedDate(task.due ?? undefined)}
                        onchange={(e) => {
                            const val = e.currentTarget.value;
                            updateTask(task.id, t => ({...t, due: val || undefined}));
                        }}
                    />
                </div>
                
                <div class="inspector-field">
                    <label for={`duration-${task.id}`}>Duration (min)</label>
                    <input
                        id={`duration-${task.id}`}
                        type="number"
                        placeholder="Unset"
                        value={task.est ?? ""}
                        onchange={(e) => {
                            const val = e.currentTarget.value;
                            updateTask(task.id, t => ({...t, est: val ? parseInt(val) : undefined}));
                        }}
                    />
                </div>
            {:else}
                {@const event = currentItem as AppEvent}
                <div class="inspector-field inspector-field-group">
                    <label for={`type-${event.id}`}>Type</label>
                    <select
                        id={`type-${event.id}`}
                        value={event.type}
                        onchange={(e) => {
                            const val = (e.target as HTMLSelectElement).value;
                            updateEvent(event.id, ev => ({...ev, type: val as "busy" | "info" | "plan"}));
                        }}
                    >
                        <option value="busy">Busy</option>
                        <option value="info">Informational</option>
                        <option value="plan">Plan</option>
                    </select>
                </div>
                
                <div class="inspector-field">
                    <label for={`attach-${event.id}`}>Task Attachment</label>
                    <select
                        id={`attach-${event.id}`}
                        value={event.taskId ?? ""}
                        onchange={(e) => {
                            const val = e.currentTarget.value;
                            updateEvent(event.id, ev => ({...ev, taskId: val || undefined}));
                        }}
                    >
                        <option value="">-- No task attached --</option>
                        {#each tasks as t (t.id)}
                            <option value={t.id}>{t.title}</option>
                        {/each}
                    </select>
                </div>
                
                <div class="inspector-field inspector-field-group">
                    <label for={`rrule-${event.id}`}>Repeat (RRULE)</label>
                    <input
                        id={`rrule-${event.id}`}
                        type="text"
                        placeholder="Custom RRULE (e.g. FREQ=WEEKLY)"
                        value={event.rrule ?? ""}
                        onchange={(e) => {
                            const val = e.currentTarget.value;
                            updateEvent(event.id, ev => ({...ev, rrule: val || undefined}));
                        }}
                    />
                </div>
                
                <div class="inspector-field-group">
                    <label for={`start-d-${event.id}`}>Start</label>
                    <div style="display: flex; gap: 8px;">
                        <input 
                            id={`start-d-${event.id}`}
                            type="date" 
                            class="inspector-date-input"
                            value={getFormattedDate(event.startTime)}
                            onchange={(e) => { updateEventDate(event, 'startTime', e.currentTarget.value); }}
                        />
                        {#if event.startTime.includes('T')}
                            <input 
                                type="time"
                                class="inspector-date-input"
                                value={getFormattedTime(event.startTime)}
                                onchange={(e) => { updateEventTime(event, 'startTime', e.currentTarget.value); }}
                            />
                        {/if}
                    </div>
                </div>
                
                <div class="inspector-field-group" style="margin-top: 8px;">
                    <label for={`end-d-${event.id}`}>End</label>
                    <div style="display: flex; gap: 8px;">
                        <input 
                            id={`end-d-${event.id}`}
                            type="date" 
                            class="inspector-date-input"
                            value={getFormattedDate(event.endTime)}
                            onchange={(e) => { updateEventDate(event, 'endTime', e.currentTarget.value); }}
                        />
                        {#if event.endTime.includes('T')}
                            <input 
                                type="time"
                                class="inspector-date-input"
                                value={getFormattedTime(event.endTime)}
                                onchange={(e) => { updateEventTime(event, 'endTime', e.currentTarget.value); }}
                            />
                        {/if}
                    </div>
                </div>
            {/if}
            
            <div class="inspector-actions" style="margin-top: 24px;">
                <button 
                    class="btn-danger"
                    onclick={() => {
                        if (isCurrentTask) deleteTask(currentItem.id);
                        else deleteEvent(currentItem.id);
                        onClose();
                    }}
                >
                    Delete {isCurrentTask ? 'Task' : 'Event'}
                </button>
            </div>
        </div>
    {/if}
</div>
