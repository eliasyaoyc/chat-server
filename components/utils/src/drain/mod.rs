#[cfg(feature = "tower")]
mod retain;

#[cfg(feature = "tower")]
pub use crate::retain::Retain;

use std::future::Future;
use tokio::sync::{mpsc, watch};
use crate::signal::shutdown;

/// Creates a drain channel.
///
/// The `Signal` is used to start a drain, and the `Watch` will be notified
/// when a drain is signaled.
pub fn channel() -> (Signal, Watch) {
    let (signal_tx, signal_rx) = watch::channel(());
    let (drained_tx, drained_rx) = mpsc::channel(1);

    let signal = Signal {
        drained_rx,
        signal_tx,
    };

    let watch = Watch {
        drained_tx,
        signal_rx,
    };
    (signal, watch)
}

enum Never {}

/// Send a drain command to all watchers.
#[derive(Debug)]
pub struct Signal {
    drained_rx: mpsc::Receiver<Never>,
    signal_tx: watch::Sender<()>,
}

/// Send a drain command to all watchers.
#[derive(Clone, Debug)]
pub struct Watch {
    drained_tx: mpsc::Sender<Never>,
    signal_rx: watch::Receiver<()>,
}

#[must_use = "ReleaseShutdown should be dropped explicitly to release the runtime"]
#[derive(Clone, Debug)]
pub struct ReleaseShutdown(mpsc::Sender<Never>);

impl Signal {
    /// Asynchronously signals all watchers to begin draining and waits for all
    /// handles to be dropped.
    pub async fn drain(mut self) {
        // Update the state of the signal watch so that all watchers are observe
        // the change.
        let _ = self.signal_tx.send(());

        // Wait for all watchers to release their drain handle.
        match self.drained_rx.recv().await {
            None => {}
            Some(n) => match n {},
        }
    }
}

impl Watch {
    /// Returns a `ReleaseShutdown` handle after the drain has been signaled. The
    /// handle must be dropped when a shutdown action has been completed to
    /// unblock graceful shutdowm.
    pub async fn signaled(mut self) -> ReleaseShutdown {
        // This future completes once `Signal::signal` has been invoked so that
        // the channel's state is updated.
        let _ = self.signal_rx.changed().await;

        // Return a handle that holds the drain channel, so that the signal task
        // is only notified when all handles have been dropped.
        ReleaseShutdown(self.drained_tx)
    }

    /// Return a `ReleaseShutdown` handle immediately, ignoring the release signal.
    ///
    /// This is intended to allow a task to block shutdown until is completes.
    pub fn ignore_signaled(self) -> ReleaseShutdown {
        drop(self.signal_rx);
        ReleaseShutdown(self.drained_tx)
    }

    /// Wrap a future and a callback that is triggered when drain is received.
    ///
    /// The callback receives a mutable reference to the original future, and
    /// should be used to trigger any shutdown process for it.
    pub async fn watch<A, F>(self, mut future: A, on_drain: F) -> A::Output
        where
            A: Future + Unpin,
            F: FnOnce(&mut A),
    {
        tokio::select! {
            res = &mut future => res,
            shutdown = self.signaled() => {
                on_drain(&mut future);
                shutdown.release_after(future).await
            }
        }
    }
}

impl ReleaseShutdown {
    /// Releases shutdown after `future` completes.
    pub async fn release_after<F: Future>(self, future: F) -> F::Output {
        let res = future.await;
        drop(self.0);
        res
    }
}