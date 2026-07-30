use std::rc::Rc;

use gpui::{Context, SharedString};

use super::Explorer;
use super::bulk_op::BulkDecision;

type WarningConfirmHandler = Rc<dyn Fn(&mut Explorer, &mut Context<Explorer>)>;

pub struct PendingWarning {
    pub title: SharedString,
    pub message: SharedString,
    pub confirm_label: SharedString,
    on_confirm: WarningConfirmHandler,
}

impl Explorer {
    pub fn show_warning(
        &mut self,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        confirm_label: impl Into<SharedString>,
        on_confirm: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        self.warning = Some(PendingWarning {
            title: title.into(),
            message: message.into(),
            confirm_label: confirm_label.into(),
            on_confirm: Rc::new(on_confirm),
        });
        cx.notify();
    }

    pub fn dismiss_warning(&mut self, cx: &mut Context<Self>) {
        if self.warning.take().is_some() {
            cx.notify();
        }
    }

    pub fn confirm_warning(&mut self, cx: &mut Context<Self>) {
        let Some(warning) = self.warning.take() else {
            return;
        };
        (warning.on_confirm.clone())(self, cx);
        cx.notify();
    }

    pub fn request_bulk_cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = &self.active_bulk_op {
            state.request_cancel();
        }
        cx.notify();
    }

    pub fn resolve_bulk_error(&mut self, decision: BulkDecision, cx: &mut Context<Self>) {
        if let Some(state) = &mut self.active_bulk_op {
            if let Some(pending) = &mut state.pending_error {
                pending.resolve(decision);
            }
            state.pending_error = None;
        }
        cx.notify();
    }
}
