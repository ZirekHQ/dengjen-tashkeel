use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Condvar, Mutex};

/// A bounded pool of pre-built `T`s, checked out via [`SessionPool::acquire`] and
/// returned automatically when the guard is dropped.
///
/// A value dropped while its thread is panicking is discarded instead of returned,
/// so a possibly-corrupted `T` never re-enters circulation; the pool permanently loses
/// that slot. Once every slot has been lost this way, further [`acquire`](SessionPool::acquire)
/// calls (including ones already waiting) return [`PoolExhausted`] instead of blocking forever.
pub struct SessionPool<T> {
    state: Mutex<PoolState<T>>,
    available: Condvar,
}

struct PoolState<T> {
    free: Vec<T>,
    /// Sessions not yet permanently discarded by a panic, whether free or checked out.
    /// Waiting is only ever worth it while this is non-zero.
    live: usize,
}

/// Every session in the pool was discarded after a panic; none can ever be returned.
#[derive(Debug)]
pub struct PoolExhausted;

impl fmt::Display for PoolExhausted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "session pool has no live sessions left; every slot was discarded after a panic"
        )
    }
}

impl std::error::Error for PoolExhausted {}

impl<T> SessionPool<T> {
    pub fn new(items: Vec<T>) -> Self {
        let live = items.len();
        Self {
            state: Mutex::new(PoolState { free: items, live }),
            available: Condvar::new(),
        }
    }

    /// Blocks until a `T` is free, then hands it out via an RAII guard.
    ///
    /// Returns [`PoolExhausted`] instead of blocking once no session can ever come
    /// back, whether that's already true or becomes true while this call is waiting.
    pub fn acquire(&self) -> Result<PooledGuard<'_, T>, PoolExhausted> {
        let mut state = self.lock();
        loop {
            if let Some(item) = state.free.pop() {
                return Ok(PooledGuard {
                    pool: self,
                    item: Some(item),
                });
            }
            if state.live == 0 {
                return Err(PoolExhausted);
            }
            state = self
                .available
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PoolState<T>> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn release(&self, item: T) {
        self.lock().free.push(item);
        self.available.notify_one();
    }

    /// A session was discarded after a panic instead of being returned.
    fn discard(&self) {
        let mut state = self.lock();
        state.live -= 1;
        let exhausted = state.live == 0;
        drop(state);
        if exhausted {
            // No one will ever release another session; every waiter needs to observe that.
            self.available.notify_all();
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().free.len()
    }
}

pub struct PooledGuard<'a, T> {
    pool: &'a SessionPool<T>,
    item: Option<T>,
}

impl<T> Deref for PooledGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.item.as_ref().expect("item taken only on drop")
    }
}

impl<T> DerefMut for PooledGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.item.as_mut().expect("item taken only on drop")
    }
}

impl<T> Drop for PooledGuard<'_, T> {
    fn drop(&mut self) {
        let Some(item) = self.item.take() else {
            return;
        };
        if std::thread::panicking() {
            log::error!(
                "dropping pooled item after a panic on this thread; pool permanently loses one slot"
            );
            self.pool.discard();
        } else {
            self.pool.release(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn acquire_hands_out_a_pooled_value() {
        let pool = SessionPool::new(vec![42]);

        let guard = pool.acquire().unwrap();

        assert_eq!(*guard, 42);
    }

    #[test]
    fn dropping_guard_returns_value_for_reuse() {
        let pool = SessionPool::new(vec![7]);

        {
            let guard = pool.acquire().unwrap();
            assert_eq!(*guard, 7);
        }
        assert_eq!(pool.len(), 1);

        let guard = pool.acquire().unwrap();
        assert_eq!(*guard, 7);
    }

    #[test]
    fn acquire_blocks_until_a_value_is_returned() {
        let pool = Arc::new(SessionPool::new(vec![1]));
        let held = pool.acquire().unwrap();
        let (tx, rx) = mpsc::channel();

        let pool_clone = Arc::clone(&pool);
        let handle = std::thread::spawn(move || {
            let _guard = pool_clone.acquire().unwrap();
            tx.send(()).unwrap();
        });

        assert_eq!(
            rx.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout),
            "acquire() should block while the only value is checked out"
        );

        drop(held);

        rx.recv_timeout(Duration::from_secs(1))
            .expect("acquire() should unblock once the value is returned");
        handle.join().unwrap();
    }

    #[test]
    fn dropping_a_guard_during_a_panic_discards_the_value_instead_of_returning_it() {
        let pool = Arc::new(SessionPool::new(vec![1]));

        let pool_clone = Arc::clone(&pool);
        let handle = std::thread::spawn(move || {
            let _guard = pool_clone.acquire().unwrap();
            panic!("simulated failure while holding a pooled value");
        });
        let _ = handle.join();

        assert_eq!(
            pool.len(),
            0,
            "a value dropped during a panic must not return to the pool"
        );
    }

    #[test]
    fn acquire_errors_immediately_once_every_session_has_been_lost_to_a_panic() {
        let pool = Arc::new(SessionPool::new(vec![1]));

        let pool_clone = Arc::clone(&pool);
        let handle = std::thread::spawn(move || {
            let _guard = pool_clone.acquire().unwrap();
            panic!("simulated failure while holding the only pooled value");
        });
        let _ = handle.join();

        let err = match pool.acquire() {
            Err(e) => e,
            Ok(_) => panic!("no session can ever be returned"),
        };
        assert!(err.to_string().contains("no live sessions"));
    }

    #[test]
    fn blocked_waiters_are_woken_with_an_error_once_the_last_session_is_lost() {
        let pool = Arc::new(SessionPool::new(vec![1]));
        let (holder_ready_tx, holder_ready_rx) = mpsc::channel();
        let (panic_now_tx, panic_now_rx) = mpsc::channel::<()>();

        let holder_pool = Arc::clone(&pool);
        let holder = std::thread::spawn(move || {
            let _guard = holder_pool.acquire().unwrap();
            holder_ready_tx.send(()).unwrap();
            let _ = panic_now_rx.recv();
            panic!("simulated failure while holding the last pooled value");
        });
        holder_ready_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("holder thread should acquire the only session");

        let (waiter_result_tx, waiter_result_rx) = mpsc::channel();
        let waiter_pool = Arc::clone(&pool);
        let waiter = std::thread::spawn(move || {
            let result = waiter_pool.acquire();
            waiter_result_tx.send(result.is_err()).unwrap();
        });

        assert_eq!(
            waiter_result_rx.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout),
            "the waiter should still be blocked while a session might come back"
        );

        panic_now_tx.send(()).unwrap();
        let _ = holder.join();

        let waiter_saw_error = waiter_result_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("the waiter should wake up once the pool is permanently empty");
        assert!(
            waiter_saw_error,
            "the woken waiter should get an error, not a value"
        );
        waiter.join().unwrap();
    }
}
