<script lang="ts">
    import { notificationStore, type NotificationType } from '$lib/notifications.svelte';

    const NOTIFICATION_TIMEOUT_MS = 5000;
    const NOTIFICATION_FLASH_DURATION_MS = 150;

    function getColors(type: NotificationType) {
        switch (type) {
            case 'error':
                return {
                    bg: 'rgba(220, 38, 38, 0.8)',
                    border: 'rgba(248, 113, 113, 0.5)'
                };
            case 'success':
                return {
                    bg: 'rgba(22, 163, 74, 0.8)',
                    border: 'rgba(74, 222, 128, 0.5)'
                };
            default:
                return {
                    bg: 'rgba(30, 41, 59, 0.8)',
                    border: 'rgba(71, 85, 105, 0.5)'
                };
        }
    }

    function autoDismiss(node: HTMLElement, type: NotificationType) {
        let timer: ReturnType<typeof setTimeout>;
        if (type !== 'error') {
            timer = setTimeout(() => {
                const id = node.id.replace('notif-', '');
                notificationStore.dismiss(id);
            }, NOTIFICATION_TIMEOUT_MS);
        }
        return {
            destroy() {
                if (timer) clearTimeout(timer);
            }
        };
    }

    async function handleClick(id: string, message: string) {
        try {
            await navigator.clipboard.writeText(message);
            const el = document.getElementById(`notif-${id}`);
            if (el) {
                const oldBg = el.style.background;
                el.style.background = 'rgba(255, 255, 255, 0.2)';
                setTimeout(() => {
                    el.style.background = oldBg || '';
                }, NOTIFICATION_FLASH_DURATION_MS);
            }
        } catch (err) {
            console.error('Failed to copy notification:', err);
        }
    }
</script>

<div class="notifications-container">
    {#each notificationStore.notifications as notification (notification.id)}
        {@const colors = getColors(notification.type)}
        <div
            id={`notif-${notification.id}`}
            class="notification-item {notification.exiting ? 'exiting' : ''}"
            style="background: {colors.bg}; border: 1px solid {colors.border};"
            use:autoDismiss={notification.type}
            role="alert"
        >
            <div
                class="notification-btn"
                title="Click to copy"
                role="button"
                tabindex="0"
                onkeydown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        void handleClick(notification.id, notification.message);
                    }
                }}
                onclick={() => handleClick(notification.id, notification.message)}
            >
                <span class="notification-msg">{notification.message}</span>
                <button
                    class="notification-dismiss"
                    title="Dismiss"
                    onclick={(e) => {
                        e.stopPropagation();
                        notificationStore.dismiss(notification.id);
                    }}
                >
                    &times;
                </button>
            </div>
        </div>
    {/each}
</div>

<style>
    .notifications-container {
        position: fixed;
        bottom: 24px;
        right: 24px;
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: 12px;
        z-index: 9999;
        pointer-events: none;
    }

    .notification-item {
        font-family: inherit;
        text-align: left;
        color: #ffffff;
        padding: 12px 20px;
        border-radius: 8px;
        font-size: 0.95rem;
        font-weight: 500;
        box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.3), 0 8px 10px -6px rgba(0, 0, 0, 0.2);
        backdrop-filter: blur(8px);
        -webkit-backdrop-filter: blur(8px);
        pointer-events: auto;
        transition: background 0.15s ease;
        animation: notify-slide-in-right 0.5s cubic-bezier(0.16, 1, 0.3, 1) forwards;
        max-width: 350px;
        word-break: break-word;
        overflow: hidden;
    }

    .notification-item.exiting {
        animation: notify-fade-out-left 0.5s ease forwards;
    }

    .notification-btn {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        width: 100%;
        outline: none;
        cursor: pointer;
        background: transparent;
        border: none;
        color: inherit;
        padding: 0;
        font-family: inherit;
        font-size: inherit;
        text-align: left;
    }

    .notification-msg {
        flex: 1;
    }

    .notification-dismiss {
        background: transparent;
        border: none;
        color: currentColor;
        cursor: pointer;
        padding: 4px;
        opacity: 0.7;
        font-size: 1.6rem;
        line-height: 1;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 4px;
    }

    @keyframes notify-slide-in-right {
        from { opacity: 0; transform: translateX(50px) scale(0.95); }
        to { opacity: 1; transform: translateX(0) scale(1); }
    }
    @keyframes notify-fade-out-left {
        0% { opacity: 1; transform: translateX(0); max-height: 500px; padding-top: 12px; padding-bottom: 12px; margin-top: 0; border-width: 1px; }
        40% { opacity: 0; transform: translateX(-30px); max-height: 500px; padding-top: 12px; padding-bottom: 12px; margin-top: 0; border-width: 1px; }
        100% { opacity: 0; transform: translateX(-30px); max-height: 0; padding-top: 0; padding-bottom: 0; margin-top: -12px; border-width: 0; }
    }
</style>
