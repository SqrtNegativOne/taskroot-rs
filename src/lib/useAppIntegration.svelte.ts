import { onMount } from 'svelte';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen, emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
import { store } from './store.svelte';
import type { AppTask } from './domain';

// Basic sigil parser replacement for the example
function parseSigils(taskName: string) {
    // A simplified version. The original taskroot parses priority, tags, est, due.
    return { cleanTitle: taskName, properties: { priority: undefined, tags: [], duration: undefined, day: undefined } };
}

export function useAppIntegration() {
    onMount(() => {
        const appWindow = getCurrentWindow();
        const isLauncher = appWindow.label === 'launcher';

        let unlistenCommands: (() => void) | undefined;
        let unlistenDeepLink: (() => void) | undefined;
        let unlistenData: (() => void) | undefined;

        if (!isLauncher) {
            // --- MAIN WINDOW LOGIC ---

            // 1. Listen for commands from the Launcher
            listen<{ action: string; payload?: Record<string, string> }>('launcher-command', async (event) => {
                const cmdData = event.payload;
                if (!cmdData || typeof cmdData !== 'object') return;

                const { action, payload } = cmdData;

                switch (action) {
                    case 'RESET_MINITRACKER':
                        // TODO: handle mini tracker
                        break;
                    case 'PLAN_TASK':
                    case 'ADD_TASK': {
                        if (!payload?.taskName) break;
                        const { cleanTitle, properties } = parseSigils(payload.taskName);
                        const newTask: AppTask = {
                            id: crypto.randomUUID(),
                            title: cleanTitle || 'New Task',
                            status: 'todo',
                            priority: properties.priority,
                            tags: properties.tags,
                            est: properties.duration,
                            due: properties.day as any
                        };
                        await store.addTask(newTask);
                        // navigate('/plan') ...
                        break;
                    }
                    case 'DO_TASK': {
                        if (!payload?.taskName) break;
                        const { cleanTitle, properties } = parseSigils(payload.taskName);
                        const newTask: AppTask = {
                            id: crypto.randomUUID(),
                            title: cleanTitle || 'New Task',
                            status: 'doing',
                            priority: properties.priority,
                            tags: properties.tags,
                            est: properties.duration,
                            due: properties.day as any
                        };
                        await store.addTask(newTask);
                        // navigate('/do') ...
                        break;
                    }
                    case 'DO_TASK_EXISTING': {
                        if (!payload?.taskId) break;
                        await store.updateTask(payload.taskId, t => ({ ...t, status: t.status === 'doing' ? 'todo' : 'doing' }));
                        // navigate('/do') ...
                        break;
                    }
                    case 'NAVIGATE':
                        if (payload?.route) {
                            // navigate(`/${payload.route}`)
                        }
                        break;
                }

                // Restore main window after executing command
                await invoke('window_restore_main');
            }).then(un => { unlistenCommands = un; });

            // 2. Listen for deep links
            onOpenUrl((urls) => {
                for (const url of urls) {
                    if (url.startsWith('taskroot://')) {
                        const route = url.replace('taskroot://', '').replace(/\/$/, '');
                        // navigate(`/${route}`);
                    }
                }
            }).then(un => { unlistenDeepLink = un; });

            // 3. Emit data to launcher whenever tasks/events change (using Svelte effects in the component)
            // This is better done in the layout using an effect, see +layout.svelte

        } else {
            // --- LAUNCHER WINDOW LOGIC ---

            // Listen for data updates from main window
            listen<{ tasks: any[]; events: any[] }>('launcher-data-update', (event) => {
                // Update local launcher state with tasks and events
                console.log("Launcher received data sync:", event.payload);
            }).then(un => { unlistenData = un; });
        }

        return () => {
            if (unlistenCommands) unlistenCommands();
            if (unlistenDeepLink) unlistenDeepLink();
            if (unlistenData) unlistenData();
        };
    });
}
