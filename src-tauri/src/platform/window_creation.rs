//! Keep native webview creation off the event-loop thread on Windows.

pub(super) async fn create_off_thread<T: Send + 'static>(
    create: impl FnOnce() -> tauri::Result<T> + Send + 'static,
) -> tauri::Result<T> {
    tauri::async_runtime::spawn_blocking(create).await?
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::create_off_thread;

    #[tokio::test(flavor = "current_thread")]
    async fn window_creation_can_wait_for_the_event_loop_without_blocking_it() {
        let (request, requested) = tokio::sync::oneshot::channel();
        let (resume, resumed) = mpsc::channel();
        let creation = tokio::spawn(create_off_thread(move || {
            request.send(()).unwrap();
            resumed
                .recv_timeout(Duration::from_secs(5))
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::TimedOut, error))?;
            Ok("effect")
        }));

        // Native creation needs the event loop to service requests before it
        // can return. Model that handshake without launching a desktop window.
        tokio::time::timeout(Duration::from_secs(2), requested)
            .await
            .unwrap()
            .unwrap();
        assert!(!creation.is_finished());
        resume.send(()).unwrap();
        assert_eq!(creation.await.unwrap().unwrap(), "effect");
    }

    #[tokio::test]
    async fn a_failed_creation_is_reported_to_the_caller() {
        let result = create_off_thread(|| Err::<(), _>(tauri::Error::WindowNotFound)).await;

        assert!(matches!(result, Err(tauri::Error::WindowNotFound)));
    }
}
