use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::StreamExt;
use futures::channel::{mpsc, oneshot};
use gpui::{Context, SharedString, Task};

use super::Explorer;

#[derive(Clone, Copy)]
pub enum BulkDecision {
    Skip,
    SkipAll,
    Retry,
    Cancel,
}

pub struct BulkItem {
    pub label: SharedString,
    perform: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
}

impl BulkItem {
    pub fn new(
        label: impl Into<SharedString>,
        perform: impl Fn() -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            perform: Box::new(perform),
        }
    }
}

pub struct PendingBulkError {
    pub label: SharedString,
    pub message: String,
    respond: Option<oneshot::Sender<BulkDecision>>,
}

impl PendingBulkError {
    pub fn resolve(&mut self, decision: BulkDecision) {
        if let Some(respond) = self.respond.take() {
            let _ = respond.send(decision);
        }
    }
}

pub struct BulkOpState {
    pub title: SharedString,
    pub total: usize,
    pub completed: usize,
    pub current: SharedString,
    pub pending_error: Option<PendingBulkError>,
    cancel: Arc<AtomicBool>,
    _task: Task<()>,
}

impl BulkOpState {
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

enum BulkEvent {
    Progress {
        completed: usize,
        current: SharedString,
    },
    NeedsDecision {
        label: SharedString,
        message: String,
        respond: oneshot::Sender<BulkDecision>,
    },
    Finished {
        errors: Vec<(SharedString, String)>,
        cancelled: bool,
    },
}

pub fn spawn(
    explorer: &mut Explorer,
    cx: &mut Context<Explorer>,
    title: impl Into<SharedString>,
    items: Vec<BulkItem>,
    on_finished: impl FnOnce(&mut Explorer, &mut Context<Explorer>, Vec<(SharedString, String)>, bool)
    + 'static,
) {
    let title = title.into();
    let total = items.len();
    if total == 0 {
        return;
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let (tx, mut rx) = mpsc::unbounded::<BulkEvent>();

    cx.background_executor()
        .spawn(async move {
            let mut errors: Vec<(SharedString, String)> = Vec::new();
            let mut skip_all = false;
            let mut cancelled = false;
            let mut processed = 0usize;

            'items: for item in items {
                if worker_cancel.load(Ordering::SeqCst) {
                    cancelled = true;
                    break;
                }

                if tx
                    .unbounded_send(BulkEvent::Progress {
                        completed: processed,
                        current: item.label.clone(),
                    })
                    .is_err()
                {
                    break;
                }

                loop {
                    match (item.perform)() {
                        Ok(()) => break,
                        Err(message) => {
                            if skip_all {
                                errors.push((item.label.clone(), message));
                                break;
                            }

                            let (respond, decision) = oneshot::channel();
                            if tx
                                .unbounded_send(BulkEvent::NeedsDecision {
                                    label: item.label.clone(),
                                    message: message.clone(),
                                    respond,
                                })
                                .is_err()
                            {
                                cancelled = true;
                                break 'items;
                            }

                            match decision.await.unwrap_or(BulkDecision::Cancel) {
                                BulkDecision::Retry => continue,
                                BulkDecision::Skip => {
                                    errors.push((item.label.clone(), message));
                                    break;
                                }
                                BulkDecision::SkipAll => {
                                    skip_all = true;
                                    errors.push((item.label.clone(), message));
                                    break;
                                }
                                BulkDecision::Cancel => {
                                    cancelled = true;
                                    break 'items;
                                }
                            }
                        }
                    }
                }

                processed += 1;
            }

            let _ = tx.unbounded_send(BulkEvent::Finished { errors, cancelled });
        })
        .detach();

    let task = cx.spawn(async move |weak, cx| {
        while let Some(event) = rx.next().await {
            match event {
                BulkEvent::Progress { completed, current } => {
                    let _ = weak.update(cx, |explorer, cx| {
                        if let Some(state) = &mut explorer.active_bulk_op {
                            state.completed = completed;
                            state.current = current;
                        }
                        cx.notify();
                    });
                }
                BulkEvent::NeedsDecision {
                    label,
                    message,
                    respond,
                } => {
                    let _ = weak.update(cx, |explorer, cx| {
                        if let Some(state) = &mut explorer.active_bulk_op {
                            state.pending_error = Some(PendingBulkError {
                                label,
                                message,
                                respond: Some(respond),
                            });
                        }
                        cx.notify();
                    });
                }
                BulkEvent::Finished { errors, cancelled } => {
                    let _ = weak.update(cx, |explorer, cx| {
                        explorer.active_bulk_op = None;
                        on_finished(explorer, cx, errors, cancelled);
                        cx.notify();
                    });
                    break;
                }
            }
        }
    });

    explorer.active_bulk_op = Some(BulkOpState {
        title,
        total,
        completed: 0,
        current: SharedString::default(),
        pending_error: None,
        cancel,
        _task: task,
    });
    cx.notify();
}
