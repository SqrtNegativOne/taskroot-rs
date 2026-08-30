export type NotificationType = "info" | "error" | "success";

export interface NotificationData {
    id: string;
    message: string;
    type: NotificationType;
    exiting: boolean;
}

const DEBOUNCE_DELAY_MS = 600;

class NotificationStore {
    notifications = $state<NotificationData[]>([]);

    notify(message: string, type: NotificationType = "info") {
        const id = Math.random().toString(36).substring(2, 11);
        this.notifications.push({ id, message, type, exiting: false });
    }

    dismiss(id: string) {
        const n = this.notifications.find((n) => n.id === id);
        if (n) {
            n.exiting = true;
            setTimeout(() => {
                this.notifications = this.notifications.filter((n) => n.id !== id);
            }, DEBOUNCE_DELAY_MS);
        }
    }
}

export const notificationStore = new NotificationStore();
export const notify = (message: string, type: NotificationType = "info") => notificationStore.notify(message, type);
