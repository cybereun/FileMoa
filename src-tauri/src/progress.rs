//! Progress reporting and cancellation shared by the long-running commands.
//!
//! Heavy work runs on a blocking worker thread and reports through a `Ctl`:
//! it can ask whether the user pressed Cancel, and it reports throttled
//! progress that the UI shows. Only one such operation runs at a time.

use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

pub const CANCELLED: &str = "작업이 취소되었습니다. / The operation was cancelled.";
const BUSY: &str = "다른 작업이 진행 중입니다. 끝나거나 취소한 뒤 다시 시도하세요. / Another operation is running; wait for it or cancel it first.";
/// The UI does not need more than a few updates per second.
const REPORT_EVERY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// `scan`, `hash` or `move`.
    pub phase: &'static str,
    pub done: u64,
    /// 0 when the total is not known yet (a folder walk).
    pub total: u64,
    pub current: String,
}

type Sink = Box<dyn Fn(Progress) + Send + Sync>;

#[derive(Default, Clone)]
pub struct Operations(Arc<Mutex<Option<Arc<AtomicBool>>>>);

impl Operations {
    /// Start the single allowed operation. The slot is released when the
    /// returned `Ctl` is dropped, however the work ends.
    pub fn begin(&self, sink: Option<Sink>) -> Result<Ctl, String> {
        let mut slot = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if slot.is_some() {
            return Err(BUSY.into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *slot = Some(cancel.clone());
        Ok(Ctl {
            cancel,
            sink,
            last: Mutex::new(None),
            release: Some(self.0.clone()),
        })
    }

    /// Ask the running operation to stop at its next checkpoint.
    pub fn cancel(&self) {
        if let Some(cancel) = self.0.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            cancel.store(true, Ordering::SeqCst);
        }
    }
}

pub struct Ctl {
    cancel: Arc<AtomicBool>,
    sink: Option<Sink>,
    last: Mutex<Option<Instant>>,
    release: Option<Arc<Mutex<Option<Arc<AtomicBool>>>>>,
}

impl Ctl {
    /// No cancellation, no progress: what the plain (test) entry points use.
    #[cfg(test)]
    pub fn none() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            sink: None,
            last: Mutex::new(None),
            release: None,
        }
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// `Err(CANCELLED)` once the user pressed Cancel.
    pub fn check(&self) -> Result<(), String> {
        if self.cancelled() {
            Err(CANCELLED.into())
        } else {
            Ok(())
        }
    }

    /// Report progress, throttled; `force` sends it regardless (final update).
    pub fn report(&self, phase: &'static str, done: u64, total: u64, current: &str, force: bool) {
        let Some(sink) = &self.sink else { return };
        let mut last = self.last.lock().unwrap_or_else(|p| p.into_inner());
        let due = last.is_none_or(|at| at.elapsed() >= REPORT_EVERY);
        if force || due {
            *last = Some(Instant::now());
            sink(Progress {
                phase,
                done,
                total,
                current: current.to_string(),
            });
        }
    }
}

impl Drop for Ctl {
    fn drop(&mut self) {
        if let Some(slot) = self.release.take() {
            *slot.lock().unwrap_or_else(|p| p.into_inner()) = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn only_one_operation_runs_at_a_time_and_the_slot_is_released() {
        let operations = Operations::default();
        let first = operations.begin(None).expect("first");
        assert!(
            operations.begin(None).is_err(),
            "a second operation is refused"
        );
        drop(first);
        assert!(operations.begin(None).is_ok(), "the slot is free again");
    }

    #[test]
    fn cancel_reaches_the_running_operation_only() {
        let operations = Operations::default();
        operations.cancel(); // nothing running: harmless
        let running = operations.begin(None).expect("begin");
        assert!(running.check().is_ok());
        operations.cancel();
        assert!(running.cancelled());
        assert_eq!(running.check().unwrap_err(), CANCELLED);
        drop(running);
        let next = operations.begin(None).expect("next");
        assert!(!next.cancelled(), "a new operation starts un-cancelled");
    }

    #[test]
    fn progress_is_throttled_but_a_forced_report_always_goes_out() {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let ctl = Operations::default()
            .begin(Some(Box::new(move |_| {
                seen.fetch_add(1, Ordering::SeqCst);
            })))
            .expect("begin");
        for done in 0..1000 {
            ctl.report("scan", done, 0, "x", false);
        }
        assert_eq!(count.load(Ordering::SeqCst), 1, "a burst yields one update");
        ctl.report("scan", 1000, 1000, "done", true);
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
