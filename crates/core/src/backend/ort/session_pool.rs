use std::ops::{Deref, DerefMut};
use std::sync::{Condvar, Mutex};

/// A bounded pool of pre-built `T`s, checked out via [`SessionPool::acquire`] and
/// returned automatically when the guard is dropped.
///
/// A value dropped while its thread is panicking is discarded instead of returned,
/// so a possibly-corrupted `T` never re-enters circulation; the pool simply shrinks by one slot.
pub struct SessionPool<T> {
    free: Mutex<Vec<T>>,
    available: Condvar,
}

impl<T> SessionPool<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            free: Mutex::new(items),
            available: Condvar::new(),
        }
    }

    /// Blocks until a `T` is free, then hands it out via an RAII guard.
    pub fn acquire(&self) -> PooledGuard<'_, T> {
        let mut free = self.lock();
        while free.is_empty() {
            free = self.available.wait(free).unwrap_or_else(|e| e.into_inner());
        }
        let item = free.pop().expect("just checked non-empty");
        PooledGuard {
            pool: self,
            item: Some(item),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<T>> {
        self.free.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn release(&self, item: T) {
        self.lock().push(item);
        self.available.notify_one();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().len()
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

        let guard = pool.acquire();

        assert_eq!(*guard, 42);
    }

    #[test]
    fn dropping_guard_returns_value_for_reuse() {
        let pool = SessionPool::new(vec![7]);

        {
            let guard = pool.acquire();
            assert_eq!(*guard, 7);
        }
        assert_eq!(pool.len(), 1);

        let guard = pool.acquire();
        assert_eq!(*guard, 7);
    }

    #[test]
    fn acquire_blocks_until_a_value_is_returned() {
        let pool = Arc::new(SessionPool::new(vec![1]));
        let held = pool.acquire();
        let (tx, rx) = mpsc::channel();

        let pool_clone = Arc::clone(&pool);
        let handle = std::thread::spawn(move || {
            let _guard = pool_clone.acquire();
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
            let _guard = pool_clone.acquire();
            panic!("simulated failure while holding a pooled value");
        });
        let _ = handle.join();

        assert_eq!(
            pool.len(),
            0,
            "a value dropped during a panic must not return to the pool"
        );
    }
}
