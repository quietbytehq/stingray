//! Closing Stingray cleanly: cancel running transfers, stop the storage engine, exit.
//! Every step has a timeout, and a watchdog ends the process even if a step hangs.

use crate::files::transfers::{State as TransferState, TransferQueue};
use crate::rclone::{RcdState, Supervisor};
use crate::text;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// How long cancelled transfers get to wind down before the engine is stopped.
pub const CANCEL_WAIT: Duration = Duration::from_secs(5);
/// After this the process exits whatever is still going on.
pub const WATCHDOG: Duration = Duration::from_secs(15);

/// Set once quitting has begun, so the sequence runs at most once.
pub struct QuitState(pub AtomicBool);

/// What the clean stop managed to do. Returned for logs and tests.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub cancelled_transfers: usize,
    /// True when nothing was still running after the cancel wait.
    pub idle: bool,
    pub engine_stopped: bool,
}

/// Stops every transfer, then the engine. Usable without an app, for tests.
pub async fn stop_everything(queue: &TransferQueue, supervisor: &Supervisor) -> Report {
    let mut report = Report::default();
    for item in queue.snapshot() {
        if matches!(item.state, TransferState::Queued | TransferState::Running) {
            let _ = queue.cancel(item.id).await;
            report.cancelled_transfers += 1;
        }
    }
    let deadline = Instant::now() + CANCEL_WAIT;
    while queue.active() > 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    report.idle = queue.active() == 0;
    supervisor.shutdown().await;
    report.engine_stopped = matches!(supervisor.state(), RcdState::Stopped);
    report
}

/// Called when the main window is asked to close. Asks first when transfers are running.
pub fn request(app: &AppHandle) {
    let busy = app
        .try_state::<Arc<TransferQueue>>()
        .map(|queue| queue.active())
        .unwrap_or(0);
    if busy > 0 {
        use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
        let handle = app.clone();
        app.dialog()
            .message(text::quit_busy(busy))
            .title(text::QUIT_TITLE)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                text::QUIT_ANYWAY.to_owned(),
                text::KEEP_RUNNING.to_owned(),
            ))
            .show(move |confirmed| {
                if confirmed {
                    begin(&handle);
                }
            });
        return;
    }
    begin(app);
}

/// Starts the quit sequence once: hide windows at once, stop services, exit.
pub fn begin(app: &AppHandle) {
    if let Some(state) = app.try_state::<QuitState>()
        && state.0.swap(true, Ordering::SeqCst)
    {
        return;
    }
    for window in app.webview_windows().values() {
        let _ = window.hide();
    }
    std::thread::spawn(|| {
        std::thread::sleep(WATCHDOG);
        eprintln!("Stingray: quit took too long, exiting now.");
        std::process::exit(0);
    });
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        shutdown_services(&handle).await;
        handle.exit(0);
    });
}

/// Runs the clean stop against the app's services.
pub async fn shutdown_services(app: &AppHandle) {
    let (Some(queue), Some(supervisor)) = (
        app.try_state::<Arc<TransferQueue>>(),
        app.try_state::<Supervisor>(),
    ) else {
        return;
    };
    let report = stop_everything(&queue, &supervisor).await;
    if !report.engine_stopped {
        eprintln!("Stingray: the storage engine did not confirm it stopped; it is killed on exit.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_defaults_to_nothing_done() {
        let report = Report::default();
        assert_eq!(report.cancelled_transfers, 0);
        assert!(!report.engine_stopped);
    }
}
