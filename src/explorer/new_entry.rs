use gpui::{AppContext, Context, Entity, Subscription, Window};

use crate::filesystem::operations::error;
use crate::filesystem::operations::new_entry as new_entry_ops;
use crate::filesystem::operations::new_entry::NewEntryKind;
use crate::filesystem::undo_op::UndoOp;
use crate::ui::text_input::{TextInputEvent as InputEvent, TextInputState as InputState};

use super::Explorer;

pub struct NewEntryState {
    pub kind: NewEntryKind,
    pub input: Entity<InputState>,
    _subscription: Subscription,
}

impl Explorer {
    pub fn begin_new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.begin_new_entry(NewEntryKind::Folder, window, cx);
    }

    pub fn begin_new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.begin_new_entry(NewEntryKind::File, window, cx);
    }

    fn begin_new_entry(&mut self, kind: NewEntryKind, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_trash() {
            return;
        }
        let placeholder = match kind {
            NewEntryKind::Folder => "New Folder",
            NewEntryKind::File => "New File",
        };

        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        input.update(cx, |input, cx| input.focus(window, cx));

        let subscription = cx.subscribe(&input, |explorer: &mut Self, _input, event, cx| {
            if let InputEvent::PressEnter { .. } = event {
                explorer.commit_new_entry(cx);
            }
        });

        self.new_entry = Some(NewEntryState {
            kind,
            input,
            _subscription: subscription,
        });
        cx.notify();
    }

    pub fn commit_new_entry(&mut self, cx: &mut Context<Self>) {
        let Some(new_entry) = self.new_entry.take() else {
            return;
        };
        let name = new_entry.input.read(cx).value().to_string();

        if !name.is_empty() {
            let result = match new_entry.kind {
                NewEntryKind::Folder => new_entry_ops::new_folder(self.current_dir(), &name),
                NewEntryKind::File => new_entry_ops::new_file(self.current_dir(), &name),
            };
            match result {
                Ok(path) => {
                    self.op_error = None;
                    self.push_undo(
                        UndoOp::NewEntry {
                            path: path.clone(),
                            kind: new_entry.kind,
                        },
                        cx,
                    );
                    self.refresh_entries();
                    self.selected = [path.clone()].into_iter().collect();
                    self.focused_path = Some(path);
                }
                Err(errors) => self.op_error = Some(error::describe(&errors)),
            }
        }
        cx.notify();
    }

    pub fn cancel_new_entry(&mut self, cx: &mut Context<Self>) {
        self.new_entry = None;
        cx.notify();
    }
}
