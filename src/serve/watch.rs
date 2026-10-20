//! Keeps the served tree current by watching the content repository on disk.
//!
//! `notify` reports raw filesystem events from the OS (FSEvents on macOS,
//! inotify on Linux). They come in bursts — a `git checkout` touches hundreds of
//! files — so they are debounced: after the first event the watcher waits for
//! [`QUIET`] without new ones, then reloads the whole tree once and announces it
//! (ADR-0012). `tree/` is watched recursively; `.git/` only at its top level,
//! where `HEAD` and `index` change on checkout, commit and staging.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use super::State;

/// How long the disk must stay quiet before a burst of events triggers a reload.
const QUIET: Duration = Duration::from_millis(300);

/// Starts watching `tree/` and `.git/` and reloading `state` after each burst.
///
/// The returned watcher must be kept alive: dropping it stops the watch. The OS
/// callback runs on a thread of `notify`, so it only forwards a signal through
/// a channel; the debounce and the reload run in a tokio task, the reload itself
/// on the blocking pool since it reads the whole tree.
///
/// # Errors
///
/// Fails when the OS watcher cannot be created or `tree/` cannot be watched.
pub fn start(state: Arc<State>) -> Result<RecommendedWatcher> {
    let (signal, mut signals) = mpsc::unbounded_channel::<()>();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok() {
            let _ = signal.send(());
        }
    })
    .context("creating the file watcher")?;
    watcher
        .watch(&state.root.join("tree"), RecursiveMode::Recursive)
        .context("watching tree/")?;
    let git = state.root.join(".git");
    if git.is_dir() {
        watcher
            .watch(&git, RecursiveMode::NonRecursive)
            .context("watching .git/")?;
    }
    tokio::spawn(async move {
        while signals.recv().await.is_some() {
            while tokio::time::timeout(QUIET, signals.recv())
                .await
                .is_ok_and(|signal| signal.is_some())
            {}
            let state = state.clone();
            let reloaded = tokio::task::spawn_blocking(move || state.reload()).await;
            match reloaded {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("ariane serve: reload failed: {error:#}"),
                Err(error) => eprintln!("ariane serve: reload task failed: {error}"),
            }
        }
    });
    Ok(watcher)
}
