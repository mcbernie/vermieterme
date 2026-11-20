//! Finishing a change even when the client goes away (#121).
//!
//! axum drops the handler future when the connection closes. Dropped between the commit
//! and the file step, an upload would lose its file although its row is committed (the
//! [`FileGuard`](super::storage::FileGuard) removes it), and a delete would leave its
//! files behind. Running the transaction and the file step in their own task closes that
//! gap: the task is not cancelled with the request.

use std::future::Future;

use crate::error::{ApiError, ApiResult};

/// Runs `work` in its own task and waits for its result. If the request is cancelled
/// meanwhile, the task still runs to its end. A panic in the task is 500 (logged
/// without details).
pub(crate) async fn detached<T: Send + 'static>(
    work: impl Future<Output = ApiResult<T>> + Send + 'static,
) -> ApiResult<T> {
    tokio::spawn(work).await.unwrap_or_else(|error| {
        tracing::error!(
            panic = error.is_panic(),
            "Abschluss einer Änderung abgebrochen"
        );
        Err(ApiError::internal())
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use super::detached;

    #[tokio::test]
    async fn work_finishes_after_the_caller_is_dropped() {
        let done = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&done);
        let request = detached(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag.store(true, Ordering::SeqCst);
            Ok(())
        });
        // The caller gives up (client disconnect): the future is dropped mid-way.
        assert!(
            tokio::time::timeout(Duration::from_millis(5), request)
                .await
                .is_err()
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(done.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn results_and_panics_come_back() {
        assert_eq!(detached(async { Ok(7) }).await.ok(), Some(7));
        let panicked = detached::<()>(async { panic!("boom") }).await;
        assert!(panicked.is_err());
    }
}
