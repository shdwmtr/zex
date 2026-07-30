use std::path::PathBuf;

use gpui::{AppContext, Context, Entity, Subscription, Window};

use crate::filesystem::operations::{error, mv};
use crate::filesystem::undo_op::UndoOp;
use crate::ui::text_input::{TextInputEvent as InputEvent, TextInputState as InputState};

use super::Explorer;

pub struct RenameState {
    pub path: PathBuf,
    pub input: Entity<InputState>,
    _subscription: Subscription,
}

impl Explorer {
    pub fn begin_rename(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let Some(&ix) = self.entry_index.get(&path) else {
            return;
        };
        let default_value = self.entries[ix].name.clone();

        let input = cx.new(|cx| InputState::new(window, cx).default_value(default_value));
        input.update(cx, |input, cx| input.focus(window, cx));

        let subscription = cx.subscribe(&input, |explorer: &mut Self, _input, event, cx| {
            if let InputEvent::PressEnter { .. } = event {
                explorer.commit_rename(cx);
            }
        });

        self.renaming = Some(RenameState {
            path,
            input,
            _subscription: subscription,
        });
        cx.notify();
    }

    pub fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.take() else {
            return;
        };
        let new_name = renaming.input.read(cx).value().to_string();
        let current_name = renaming
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();

        if !new_name.is_empty() && new_name != current_name {
            match mv::rename(&renaming.path, &new_name) {
                Ok(new_path) => {
                    self.op_error = None;
                    self.push_undo(
                        UndoOp::Rename {
                            from: renaming.path,
                            to: new_path.clone(),
                        },
                        cx,
                    );
                    self.refresh_entries();
                    self.selected = [new_path.clone()].into_iter().collect();
                    self.focused_path = Some(new_path);
                }
                Err(errors) => self.op_error = Some(error::describe(&errors)),
            }
        }
        cx.notify();
    }

    pub fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming = None;
        cx.notify();
    }
}
