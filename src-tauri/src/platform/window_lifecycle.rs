//! Serialize each window's complete create/show/hide/toggle operation.

pub(super) struct WindowLifecycle(tokio::sync::Mutex<()>);

impl WindowLifecycle {
    pub(super) const fn new() -> Self {
        Self(tokio::sync::Mutex::const_new(()))
    }

    pub(super) async fn lock(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.0.lock().await
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::WindowLifecycle;

    struct Desktop {
        lifecycle: WindowLifecycle,
        created: AtomicBool,
        creations: AtomicUsize,
        visible: AtomicBool,
    }

    impl Desktop {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                lifecycle: WindowLifecycle::new(),
                created: AtomicBool::new(false),
                creations: AtomicUsize::new(0),
                visible: AtomicBool::new(false),
            })
        }

        async fn show(&self, creation: impl Future<Output = ()>) {
            let _operation = self.lifecycle.lock().await;
            if !self.created.load(Ordering::SeqCst) {
                self.creations.fetch_add(1, Ordering::SeqCst);
                creation.await;
                self.created.store(true, Ordering::SeqCst);
            }
            self.visible.store(true, Ordering::SeqCst);
        }

        async fn hide(&self) {
            let _operation = self.lifecycle.lock().await;
            self.visible.store(false, Ordering::SeqCst);
        }
    }

    async fn begin_creation(
        desktop: &Arc<Desktop>,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (started, starting) = tokio::sync::oneshot::channel();
        let (finish, finished) = tokio::sync::oneshot::channel();
        let desktop = Arc::clone(desktop);
        let task = tokio::spawn(async move {
            desktop
                .show(async move {
                    started.send(()).unwrap();
                    finished.await.unwrap();
                })
                .await;
        });
        starting.await.unwrap();
        (task, finish)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn overlapping_show_requests_create_only_one_window() {
        let desktop = Desktop::new();
        let (first, finish) = begin_creation(&desktop).await;
        let (queued, queuing) = tokio::sync::oneshot::channel();
        let other = Arc::clone(&desktop);
        let second = tokio::spawn(async move {
            queued.send(()).unwrap();
            other.show(std::future::ready(())).await;
        });
        queuing.await.unwrap();
        assert!(!second.is_finished());

        finish.send(()).unwrap();
        first.await.unwrap();
        second.await.unwrap();
        assert_eq!(desktop.creations.load(Ordering::SeqCst), 1);
        assert!(desktop.visible.load(Ordering::SeqCst));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_hide_during_creation_leaves_the_completed_window_hidden() {
        let desktop = Desktop::new();
        let (show, finish) = begin_creation(&desktop).await;
        let (queued, queuing) = tokio::sync::oneshot::channel();
        let other = Arc::clone(&desktop);
        let hide = tokio::spawn(async move {
            queued.send(()).unwrap();
            other.hide().await;
        });
        queuing.await.unwrap();
        assert!(!hide.is_finished());

        finish.send(()).unwrap();
        show.await.unwrap();
        hide.await.unwrap();
        assert!(desktop.created.load(Ordering::SeqCst));
        assert!(!desktop.visible.load(Ordering::SeqCst));
    }
}
