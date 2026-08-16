import { safeInvoke } from './safeInvoke.svelte';
import { ResultAsync } from 'neverthrow';
import type { AppTask, AppEvent } from './domain';

export class AppStore {
    tasks = $state<AppTask[]>([]);
    events = $state<AppEvent[]>([]);
    loaded = $state(false);
    error = $state<string | null>(null);

    async init() {
        let attempts = 0;
        while (attempts < 50) {
            const result = await ResultAsync.combine([
                safeInvoke<AppTask[]>('get_tasks'),
                safeInvoke<AppEvent[]>('get_events')
            ]);
            
            if (result.isOk()) {
                const [fetchedTasks, fetchedEvents] = result.value;
                this.tasks = fetchedTasks;
                this.events = fetchedEvents;
                this.loaded = true;
                this.error = null;
                return;
            } else {
                const e = result.error;
                const errStr = e instanceof Error ? e.toString() : String(e);
                if (errStr.includes("state not managed") || errStr.includes("not managed") || errStr.includes("not initialized")) {
                    attempts++;
                    await new Promise(r => setTimeout(r, 100));
                    continue;
                }
                this.error = `Error loading data from backend: ${errStr}`;
                return;
            }
        }
        this.error = "Error loading data from backend: Database failed to initialize in time.";
    }

    async addTask(task: AppTask) {
        this.tasks.push(task);
        const result = await safeInvoke('create_task', { task });
        if (result.isErr()) {
            this.tasks = this.tasks.filter(t => t.id !== task.id);
            throw result.error;
        }
    }

    async updateTask(id: string, updater: (t: AppTask) => AppTask) {
        const idx = this.tasks.findIndex(t => t.id === id);
        if (idx === -1) return;
        const oldTask = this.tasks[idx];
        const newTask = updater(oldTask);
        
        this.tasks[idx] = newTask;
        
        const result = await safeInvoke('update_task', { task: newTask });
        if (result.isErr()) {
            this.tasks[idx] = oldTask;
            throw result.error;
        }
    }

    async deleteTask(id: string) {
        const oldTasks = [...this.tasks];
        this.tasks = this.tasks.filter(t => t.id !== id);
        const result = await safeInvoke('delete_task', { id });
        if (result.isErr()) {
            this.tasks = oldTasks;
            throw result.error;
        }
    }

    async addEvent(event: AppEvent) {
        this.events.push(event);
        const result = await safeInvoke('create_event', { event });
        if (result.isErr()) {
            this.events = this.events.filter(ev => ev.id !== event.id);
            throw result.error;
        }
    }

    async updateEvent(id: string, updater: (ev: AppEvent) => AppEvent) {
        const idx = this.events.findIndex(e => e.id === id);
        if (idx === -1) return;
        const oldEvent = this.events[idx];
        const newEvent = updater(oldEvent);
        
        this.events[idx] = newEvent;
        
        const result = await safeInvoke('update_event', { event: newEvent });
        if (result.isErr()) {
            this.events[idx] = oldEvent;
            throw result.error;
        }
    }

    async deleteEvent(id: string) {
        const oldEvents = [...this.events];
        this.events = this.events.filter(e => e.id !== id);
        const result = await safeInvoke('delete_event', { id });
        if (result.isErr()) {
            this.events = oldEvents;
            throw result.error;
        }
    }
}

export const store = new AppStore();
