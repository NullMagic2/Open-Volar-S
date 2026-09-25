//! Bound retained GPU submissions; completion is checked by the graphics API.
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;

const LIMIT: usize = 3;
#[derive(Clone, Default)]
pub struct Work(Arc<AtomicUsize>);
pub struct Lease(Work);

impl Work {
    fn available(&self) -> bool { self.0.load(Ordering::Acquire) < LIMIT }

    pub fn ready(&self, device: &wgpu::Device) -> Result<bool, wgpu::PollError> {
        self.ready_with(|| device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(2)),
        }).map(|_| ()))
    }

    // Only a full queue needs a wait. Polling delivers completion callbacks
    // before making any decision; elapsed CPU scheduling time is irrelevant.
    fn ready_with<E>(&self, wait: impl FnOnce() -> Result<(), E>) -> Result<bool, E> {
        if !self.available() { wait()?; }
        Ok(self.available())
    }

    pub fn reserve(&self) -> Result<Lease, &'static str> {
        self.0.fetch_update(Ordering::AcqRel, Ordering::Acquire,
            |pending| (pending < LIMIT).then_some(pending + 1))
            .map(|_| Lease(self.clone()))
            .map_err(|_| "GPU submission limit reached")
    }
}
impl Drop for Lease {
    fn drop(&mut self) { self.0.0.fetch_sub(1, Ordering::AcqRel); }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn submissions_are_bounded_until_completion() {
        let w=Work::default();let a=w.reserve().unwrap();let b=w.reserve().unwrap();let c=w.reserve().unwrap();
        assert!(!w.available());assert!(w.reserve().is_err());drop(b);assert!(w.available());
        let d=w.reserve().unwrap();drop((a,c,d));assert!(w.available());
    }
    #[test] fn completed_work_is_collected_before_deciding_to_wait_again() {
        let w=Work::default();let leases=(0..LIMIT).map(|_|w.reserve().unwrap()).collect::<Vec<_>>();
        // Model a delayed caller: the GPU completed while its callbacks awaited polling.
        assert!(w.ready_with(|| {drop(leases);Ok::<_, &str>(())}).unwrap());
        assert!(w.ready_with(|| -> Result<(), &str> {panic!("An available queue must not wait")}).unwrap());
    }
    #[test] fn a_real_wait_error_preserves_in_flight_ownership() {
        let w=Work::default();let leases=(0..LIMIT).map(|_|w.reserve().unwrap()).collect::<Vec<_>>();
        assert_eq!(w.ready_with(|| Err("timeout")),Err("timeout"));
        assert!(!w.available());assert!(w.reserve().is_err());
        drop(leases);assert!(w.available());
    }
}
