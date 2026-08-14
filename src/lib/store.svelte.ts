import { invoke } from '@tauri-apps/api/core';
import type { AppTask, AppEvent } from './domain';

export class AppStore {
    tasks = $state<AppTask[]>([]);
    events = $state<AppEvent[]>([]);
    loaded = $state(false);
    error = $state<string | null>(null);

    async init() {
        let attempts = 0;
        while (attempts < 50) {
            try {
                const [fetchedTasks, fetchedEvents] = await Promise.all([
                    invoke<AppTask[]>('get_tasks'),
                    invoke<AppEvent[]>('get_events')
                ]);
                
                this.tasks = fetchedTasks;
                this.events = fetchedEvents;
                this.loaded = true;
                this.error = null;
                return;
            } catch (e: unknown) {
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
        try {
            await invoke('create_task', { task });
        } catch (e) {
            this.tasks = this.tasks.filter(t => t.id !== task.id);
            throw e;
        }
    }

    async updateTask(id: string, updater: (t: AppTask) => AppTask) {
        const idx = this.tasks.findIndex(t => t.id === id);
        if (idx === -1) return;
        const oldTask = this.tasks[idx];
        const newTask = updater(oldTask);
        
        this.tasks[idx] = newTask;
        
        try {
            await invoke('update_task', { task: newTask });
        } catch (e) {
            this.tasks[idx] = oldTask;
            throw e;
        }
    }

    async deleteTask(id: string) {
        const oldTasks = [...this.tasks];
        this.tasks = this.tasks.filter(t => t.id !== id);
        try {
            await invoke('delete_task', { id });
        } catch (e) {
            this.tasks = oldTasks;
            throw e;
        }
    }

    async addEvent(event: AppEvent) {
        this.events.push(event);
        try {
            await invoke('create_event', { event });
        } catch (e) {
            this.events = this.events.filter(ev => ev.id !== event.id);
            throw e;
        }
    }

    async updateEvent(id: string, updater: (ev: AppEvent) => AppEvent) {
        const idx = this.events.findIndex(e => e.id === id);
        if (idx === -1) return;
        const oldEvent = this.events[idx];
        const newEvent = updater(oldEvent);
        
        this.events[idx] = newEvent;
        
        try {
            await invoke('update_event', { event: newEvent });
        } catch (e) {
            this.events[idx] = oldEvent;
            throw e;
        }
    }

    async deleteEvent(id: string) {
        const oldEvents = [...this.events];
        this.events = this.events.filter(e => e.id !== id);
        try {
            await invoke('delete_event', { id });
        } catch (e) {
            this.events = oldEvents;
            throw e;
        }
    }
}

export const store = new AppStore();
