//! Bounded CPU ownership of GPU image leases. GPU completion callbacks retain
//! a lease until commands finish, so a free slot cannot still be in flight.
use std::{collections::VecDeque, sync::Arc};
pub const CAPACITY: usize = 3;

pub struct Pool<T> {
    slots: [Option<Arc<T>>; CAPACITY],
}
impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
    }
}
impl<T> Pool<T> {
    pub fn acquire(
        &mut self,
        valid: impl Fn(&T) -> bool,
        create: impl FnOnce() -> T,
    ) -> Option<Arc<T>> {
        let slot = self
            .slots
            .iter_mut()
            .find(|s| s.as_ref().is_none_or(|s| Arc::strong_count(s) == 1))?;
        if slot.as_ref().is_none_or(|s| !valid(s)) {
            *slot = Some(Arc::new(create()));
        }
        slot.clone()
    }
}
pub struct Timed<T> {
    pub epoch: u64,
    pub due: i64,
    pub value: T,
}
pub struct Ready<T> {
    frames: VecDeque<Timed<T>>,
}
impl<T> Default for Ready<T> {
    fn default() -> Self {
        Self {
            frames: VecDeque::new(),
        }
    }
}
impl<T> Ready<T> {
    pub fn clear(&mut self) {
        self.frames.clear();
    }
    pub fn publish(&mut self, frame: Timed<T>) {
        self.frames.retain(|f| f.epoch == frame.epoch);
        if self.frames.len() == CAPACITY - 1 {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }
    pub fn select(&mut self, epoch: u64, target: i64) -> Option<Timed<T>> {
        self.frames.retain(|f| f.epoch == epoch);
        let i = self.frames.iter().rposition(|f| f.due <= target)?;
        self.frames.drain(..i);
        self.frames.pop_front()
    }
    pub fn discard_obsolete(&mut self, epoch: u64, target: i64) {
        self.frames.retain(|f| f.epoch == epoch);
        while self.frames.len() > 1 && self.frames[1].due <= target {
            self.frames.pop_front();
        }
    }
    pub fn len(&self) -> usize {
        self.frames.len()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn in_flight_and_display_leases_prevent_reuse() {
        let mut pool = Pool::default();
        let a = pool.acquire(|_| true, || 1).unwrap();
        let b = pool.acquire(|_| true, || 2).unwrap();
        let c = pool.acquire(|_| true, || 3).unwrap();
        let gpu_completion = a.clone();
        drop(a);
        assert!(pool.acquire(|_| true, || 4).is_none());
        drop(gpu_completion);
        assert_eq!(*pool.acquire(|_| true, || 4).unwrap(), 1);
        drop((b, c));
    }
    #[test]
    fn resize_does_not_replace_in_flight_generation() {
        let mut pool = Pool::default();
        let old = pool.acquire(|v| *v == 1, || 1).unwrap();
        let new = pool.acquire(|v| *v == 2, || 2).unwrap();
        assert_eq!(*old, 1);
        assert_eq!(*new, 2);
    }
    #[test]
    fn clock_selection_keeps_future_and_discards_stale_frames() {
        let mut q = Ready::default();
        for due in [10, 20] {
            q.publish(Timed {
                epoch: 1,
                due,
                value: due,
            });
        }
        assert!(q.select(1, 9).is_none());
        assert_eq!(q.select(1, 15).unwrap().value, 10);
        assert!(q.select(1, 19).is_none());
        assert_eq!(q.select(1, 21).unwrap().value, 20);
    }
    #[test]
    fn new_epoch_and_bounded_handoff_never_return_old_channel() {
        let mut q = Ready::default();
        q.publish(Timed {
            epoch: 1,
            due: 10,
            value: 1,
        });
        assert!(q.select(2, 100).is_none());
        for due in [10, 20, 30] {
            q.publish(Timed {
                epoch: 2,
                due,
                value: due,
            });
        }
        assert_eq!(q.len(), 2);
        q.discard_obsolete(2, 30);
        assert_eq!(q.len(), 1);
        assert_eq!(q.select(2, 30).unwrap().value, 30);
    }
    #[test]
    fn dropping_stale_ready_frame_still_keeps_gpu_lease() {
        let mut pool = Pool::default();
        let mut q = Ready::default();
        let a = pool.acquire(|_| true, || 1).unwrap();
        let gpu = a.clone();
        q.publish(Timed {
            epoch: 1,
            due: 1,
            value: a,
        });
        let b = pool.acquire(|_| true, || 2).unwrap();
        let c = pool.acquire(|_| true, || 3).unwrap();
        q.clear();
        assert!(pool.acquire(|_| true, || 4).is_none());
        drop(gpu);
        assert!(pool.acquire(|_| true, || 4).is_some());
        drop((b, c));
    }
}
