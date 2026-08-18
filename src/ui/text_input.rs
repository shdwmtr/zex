use std::ops::Range;
use std::time::Duration;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, DispatchPhase, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, HitboxBehavior, Hsla, InteractiveElement,
    IntoElement, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Point, RenderOnce, SharedString, Styled, Task, TextRun, UTF16Selection,
    Window, actions, canvas, div, fill, point, size,
};

use crate::theme;

actions!(
    zex_input,
    [
        Escape,
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteWordRight,
        MoveLeft,
        MoveRight,
        MoveWordLeft,
        MoveWordRight,
        MoveHome,
        MoveEnd,
        SelectLeft,
        SelectRight,
        SelectWordLeft,
        SelectWordRight,
        SelectHome,
        SelectEnd,
        SelectAll,
        Copy,
        Cut,
        Paste,
        SubmitEnter,
    ]
);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("Input")),
        KeyBinding::new("delete", Delete, Some("Input")),
        KeyBinding::new("ctrl-backspace", DeleteWordLeft, Some("Input")),
        KeyBinding::new("ctrl-delete", DeleteWordRight, Some("Input")),
        KeyBinding::new("left", MoveLeft, Some("Input")),
        KeyBinding::new("right", MoveRight, Some("Input")),
        KeyBinding::new("ctrl-left", MoveWordLeft, Some("Input")),
        KeyBinding::new("ctrl-right", MoveWordRight, Some("Input")),
        KeyBinding::new("shift-left", SelectLeft, Some("Input")),
        KeyBinding::new("shift-right", SelectRight, Some("Input")),
        KeyBinding::new("ctrl-shift-left", SelectWordLeft, Some("Input")),
        KeyBinding::new("ctrl-shift-right", SelectWordRight, Some("Input")),
        KeyBinding::new("home", MoveHome, Some("Input")),
        KeyBinding::new("end", MoveEnd, Some("Input")),
        KeyBinding::new("shift-home", SelectHome, Some("Input")),
        KeyBinding::new("shift-end", SelectEnd, Some("Input")),
        KeyBinding::new("ctrl-a", SelectAll, Some("Input")),
        KeyBinding::new("ctrl-c", Copy, Some("Input")),
        KeyBinding::new("ctrl-x", Cut, Some("Input")),
        KeyBinding::new("ctrl-v", Paste, Some("Input")),
        KeyBinding::new("enter", SubmitEnter, Some("Input")),
        KeyBinding::new("escape", Escape, Some("Input")),
    ]);
}

pub enum TextInputEvent {
    PressEnter {},
    Changed {},
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

fn byte_to_utf16(content: &str, byte_offset: usize) -> usize {
    utf16_len(&content[..byte_offset])
}

fn utf16_to_byte(content: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    for (byte_idx, ch) in content.char_indices() {
        if utf16_count >= utf16_offset {
            return byte_idx;
        }
        utf16_count += ch.len_utf16();
    }
    content.len()
}

fn shape(content: &str, font: &gpui::Font, font_size: Pixels, window: &Window) -> gpui::ShapedLine {
    window.text_system().shape_line(
        content.to_string().into(),
        font_size,
        &[TextRun {
            len: content.len(),
            font: font.clone(),
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }],
        None,
    )
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum CharKind {
    Space,
    Word,
    Punct,
}

fn char_kind(c: char) -> CharKind {
    if c.is_whitespace() {
        CharKind::Space
    } else if c.is_alphanumeric() || c == '_' {
        CharKind::Word
    } else {
        CharKind::Punct
    }
}

const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(530);

pub struct TextInputState {
    content: String,
    selected_range: Range<usize>,
    selection_reversed: bool,
    is_selecting: bool,
    placeholder: SharedString,
    focus_handle: FocusHandle,
    marked_range: Option<Range<usize>>,
    last_bounds: Option<Bounds<Pixels>>,
    blink_visible: bool,
    blink_task: Option<Task<()>>,
}

impl TextInputState {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            content: String::new(),
            selected_range: 0..0,
            selection_reversed: false,
            is_selecting: false,
            placeholder: SharedString::default(),
            focus_handle: cx.focus_handle(),
            marked_range: None,
            last_bounds: None,
            blink_visible: true,
            blink_task: None,
        }
    }

    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.content = value.into();
        let len = self.content.len();
        self.selected_range = len..len;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        self.blink_visible = true;
        self.blink_task = Some(cx.spawn(async move |weak, cx| {
            loop {
                cx.background_executor().timer(CURSOR_BLINK_INTERVAL).await;
                let alive = weak
                    .update(cx, |state, cx| {
                        state.blink_visible = !state.blink_visible;
                        cx.notify();
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        }));
    }

    pub fn value(&self) -> SharedString {
        self.content.clone().into()
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    pub fn set_content(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.content = value.into();
        let len = self.content.len();
        self.selected_range = len..len;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.emit(TextInputEvent::Changed {});
        cx.notify();
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn prev_boundary(&self, from: usize) -> usize {
        if from == 0 {
            return 0;
        }
        let mut idx = from - 1;
        while idx > 0 && !self.content.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    fn next_boundary(&self, from: usize) -> usize {
        if from >= self.content.len() {
            return self.content.len();
        }
        let mut idx = from + 1;
        while idx < self.content.len() && !self.content.is_char_boundary(idx) {
            idx += 1;
        }
        idx
    }

    fn prev_word_boundary(&self, from: usize) -> usize {
        let head = &self.content[..from];
        let mut chars = head.char_indices().rev().peekable();
        while matches!(chars.peek(), Some((_, c)) if char_kind(*c) == CharKind::Space) {
            chars.next();
        }
        let kind = chars.peek().map(|(_, c)| char_kind(*c));
        let mut start = 0;
        while let Some(&(idx, c)) = chars.peek() {
            if Some(char_kind(c)) == kind {
                start = idx;
                chars.next();
            } else {
                break;
            }
        }
        start
    }

    fn next_word_boundary(&self, from: usize) -> usize {
        let rest = &self.content[from..];
        let mut chars = rest.char_indices().peekable();
        while matches!(chars.peek(), Some((_, c)) if char_kind(*c) == CharKind::Space) {
            chars.next();
        }
        let kind = chars.peek().map(|(_, c)| char_kind(*c));
        let mut end = rest.len();
        while let Some(&(idx, c)) = chars.peek() {
            if Some(char_kind(c)) == kind {
                chars.next();
            } else {
                end = idx;
                break;
            }
        }
        from + end
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn delete_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        self.content.replace_range(range.clone(), "");
        self.selected_range = range.start..range.start;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.emit(TextInputEvent::Changed {});
        cx.notify();
    }

    fn move_left(&mut self, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let target = self.prev_boundary(self.selected_range.start);
            self.move_to(target, cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn move_right(&mut self, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let target = self.next_boundary(self.selected_range.end);
            self.move_to(target, cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn move_word_left(&mut self, cx: &mut Context<Self>) {
        let target = self.prev_word_boundary(self.cursor_offset());
        self.move_to(target, cx);
    }

    fn move_word_right(&mut self, cx: &mut Context<Self>) {
        let target = self.next_word_boundary(self.cursor_offset());
        self.move_to(target, cx);
    }

    fn move_home(&mut self, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn move_end(&mut self, cx: &mut Context<Self>) {
        let len = self.content.len();
        self.move_to(len, cx);
    }

    fn select_left(&mut self, cx: &mut Context<Self>) {
        let target = self.prev_boundary(self.cursor_offset());
        self.select_to(target, cx);
    }

    fn select_right(&mut self, cx: &mut Context<Self>) {
        let target = self.next_boundary(self.cursor_offset());
        self.select_to(target, cx);
    }

    fn select_word_left(&mut self, cx: &mut Context<Self>) {
        let target = self.prev_word_boundary(self.cursor_offset());
        self.select_to(target, cx);
    }

    fn select_word_right(&mut self, cx: &mut Context<Self>) {
        let target = self.next_word_boundary(self.cursor_offset());
        self.select_to(target, cx);
    }

    fn select_home(&mut self, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    fn select_end(&mut self, cx: &mut Context<Self>) {
        let len = self.content.len();
        self.select_to(len, cx);
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        self.selection_reversed = false;
        self.selected_range = 0..self.content.len();
        cx.notify();
    }

    fn backspace(&mut self, cx: &mut Context<Self>) {
        let range = if self.selected_range.is_empty() {
            let start = self.prev_boundary(self.selected_range.start);
            start..self.selected_range.start
        } else {
            self.selected_range.clone()
        };
        if range.is_empty() {
            return;
        }
        self.delete_range(range, cx);
    }

    fn delete_forward(&mut self, cx: &mut Context<Self>) {
        let range = if self.selected_range.is_empty() {
            let end = self.next_boundary(self.selected_range.end);
            self.selected_range.end..end
        } else {
            self.selected_range.clone()
        };
        if range.is_empty() {
            return;
        }
        self.delete_range(range, cx);
    }

    fn delete_word_left(&mut self, cx: &mut Context<Self>) {
        let range = if self.selected_range.is_empty() {
            let start = self.prev_word_boundary(self.selected_range.start);
            start..self.selected_range.start
        } else {
            self.selected_range.clone()
        };
        if range.is_empty() {
            return;
        }
        self.delete_range(range, cx);
    }

    fn delete_word_right(&mut self, cx: &mut Context<Self>) {
        let range = if self.selected_range.is_empty() {
            let end = self.next_word_boundary(self.selected_range.end);
            self.selected_range.end..end
        } else {
            self.selected_range.clone()
        };
        if range.is_empty() {
            return;
        }
        self.delete_range(range, cx);
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.delete_range(self.selected_range.clone(), cx);
        }
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let text = text.replace('\n', " ");
        let range = self.selected_range.clone();
        self.content.replace_range(range.clone(), &text);
        let new_cursor = range.start + text.len();
        self.selected_range = new_cursor..new_cursor;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.emit(TextInputEvent::Changed {});
        cx.notify();
    }

    fn press_enter(&mut self, cx: &mut Context<Self>) {
        cx.emit(TextInputEvent::PressEnter {});
    }
}

impl EventEmitter<TextInputEvent> for TextInputState {}

impl EntityInputHandler for TextInputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let start = utf16_to_byte(&self.content, range_utf16.start);
        let end = utf16_to_byte(&self.content, range_utf16.end);
        *adjusted_range =
            Some(byte_to_utf16(&self.content, start)..byte_to_utf16(&self.content, end));
        Some(self.content.get(start..end)?.to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: byte_to_utf16(&self.content, self.selected_range.start)
                ..byte_to_utf16(&self.content, self.selected_range.end),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .clone()
            .map(|r| byte_to_utf16(&self.content, r.start)..byte_to_utf16(&self.content, r.end))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|r| utf16_to_byte(&self.content, r.start)..utf16_to_byte(&self.content, r.end))
            .or_else(|| self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        self.content.replace_range(range.clone(), text);
        let new_cursor = range.start + text.len();
        self.selected_range = new_cursor..new_cursor;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.emit(TextInputEvent::Changed {});
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|r| utf16_to_byte(&self.content, r.start)..utf16_to_byte(&self.content, r.end))
            .or_else(|| self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        self.content.replace_range(range.clone(), new_text);
        let new_marked_end = range.start + new_text.len();
        self.marked_range = Some(range.start..new_marked_end);
        let new_cursor = new_selected_range
            .map(|r| range.start + utf16_to_byte(new_text, r.end))
            .unwrap_or(new_marked_end);
        self.selected_range = new_cursor..new_cursor;
        self.selection_reversed = false;
        cx.emit(TextInputEvent::Changed {});
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let byte_offset = utf16_to_byte(&self.content, range_utf16.start);
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let shaped = shape(&self.content, &text_style.font(), font_size, window);
        let x = shaped.x_for_index(byte_offset);
        Some(Bounds::new(
            point(element_bounds.origin.x + x, element_bounds.origin.y),
            size(gpui::px(1.0), element_bounds.size.height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let local_x = point.x - bounds.origin.x;
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let shaped = shape(&self.content, &text_style.font(), font_size, window);
        Some(shaped.closest_index_for_x(local_x))
    }
}

#[derive(IntoElement)]
pub struct TextInput {
    state: Entity<TextInputState>,
}

impl TextInput {
    pub fn new(state: &Entity<TextInputState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for TextInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state;
        let snapshot = state.read(cx);
        let content = snapshot.content.clone();
        let selected_range = snapshot.selected_range.clone();
        let selection_reversed = snapshot.selection_reversed;
        let placeholder = snapshot.placeholder.clone();
        let focus_handle = snapshot.focus_handle.clone();
        let blink_visible = snapshot.blink_visible;

        let is_focused = focus_handle.is_focused(window);
        let text_style = window.text_style();
        let font = text_style.font();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();

        let show_placeholder = content.is_empty();
        let display_text: SharedString = if show_placeholder {
            placeholder.clone()
        } else {
            content.clone().into()
        };
        let display_color: Hsla = if show_placeholder {
            theme::text_faint().into()
        } else {
            theme::text_primary().into()
        };

        let shaped = window.text_system().shape_line(
            display_text.clone(),
            font_size,
            &[TextRun {
                len: display_text.len(),
                font: font.clone(),
                color: display_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );

        let cursor_offset = if selection_reversed {
            selected_range.start
        } else {
            selected_range.end
        };
        let cursor_x = if show_placeholder {
            gpui::px(0.0)
        } else {
            shaped.x_for_index(cursor_offset)
        };
        let selection_bounds = if show_placeholder || selected_range.is_empty() {
            None
        } else {
            Some((
                shaped.x_for_index(selected_range.start),
                shaped.x_for_index(selected_range.end),
            ))
        };

        let ime_state = state.clone();
        let bounds_cache_state = state.clone();
        let down_state = state.clone();
        let down_content = content.clone();
        let down_font = font.clone();
        let move_state = state.clone();
        let move_content = content.clone();
        let move_font = font.clone();
        let up_state = state.clone();

        div()
            .id("text-input")
            .track_focus(&focus_handle)
            .key_context("Input")
            .cursor(CursorStyle::IBeam)
            .h(line_height)
            .relative()
            .flex()
            .items_center()
            .on_action({
                let state = state.clone();
                move |_: &Backspace, _window, cx| state.update(cx, |s, cx| s.backspace(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &Delete, _window, cx| state.update(cx, |s, cx| s.delete_forward(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &DeleteWordLeft, _window, cx| {
                    state.update(cx, |s, cx| s.delete_word_left(cx))
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &DeleteWordRight, _window, cx| {
                    state.update(cx, |s, cx| s.delete_word_right(cx))
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveLeft, _window, cx| state.update(cx, |s, cx| s.move_left(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveRight, _window, cx| state.update(cx, |s, cx| s.move_right(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveWordLeft, _window, cx| state.update(cx, |s, cx| s.move_word_left(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveWordRight, _window, cx| {
                    state.update(cx, |s, cx| s.move_word_right(cx))
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveHome, _window, cx| state.update(cx, |s, cx| s.move_home(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &MoveEnd, _window, cx| state.update(cx, |s, cx| s.move_end(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectLeft, _window, cx| state.update(cx, |s, cx| s.select_left(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectRight, _window, cx| state.update(cx, |s, cx| s.select_right(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectWordLeft, _window, cx| {
                    state.update(cx, |s, cx| s.select_word_left(cx))
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectWordRight, _window, cx| {
                    state.update(cx, |s, cx| s.select_word_right(cx))
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectHome, _window, cx| state.update(cx, |s, cx| s.select_home(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectEnd, _window, cx| state.update(cx, |s, cx| s.select_end(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &SelectAll, _window, cx| state.update(cx, |s, cx| s.select_all(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &Copy, _window, cx| state.update(cx, |s, cx| s.copy(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &Cut, _window, cx| state.update(cx, |s, cx| s.cut(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &Paste, _window, cx| state.update(cx, |s, cx| s.paste(cx))
            })
            .on_action({
                let state = state.clone();
                move |_: &SubmitEnter, _window, cx| state.update(cx, |s, cx| s.press_enter(cx))
            })
            .child(
                canvas(
                    move |bounds, window, _cx| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                    move |bounds, hitbox, window, cx| {
                        let _ = shaped.paint(bounds.origin, line_height, window, cx);

                        if let Some((start_x, end_x)) = selection_bounds {
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(bounds.origin.x + start_x, bounds.origin.y),
                                    size(end_x - start_x, bounds.size.height),
                                ),
                                theme::text_selection_fill(),
                            ));
                        } else if is_focused && blink_visible {
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(bounds.origin.x + cursor_x, bounds.origin.y),
                                    size(gpui::px(1.0), bounds.size.height),
                                ),
                                theme::text_primary(),
                            ));
                        }

                        window.handle_input(
                            &focus_handle,
                            ElementInputHandler::new(bounds, ime_state.clone()),
                            cx,
                        );

                        window.on_mouse_event({
                            let down_state = down_state.clone();
                            let down_content = down_content.clone();
                            let down_font = down_font.clone();
                            let hitbox = hitbox.clone();
                            move |event: &MouseDownEvent, phase, window, cx| {
                                if phase == DispatchPhase::Bubble
                                    && event.button == MouseButton::Left
                                    && hitbox.is_hovered(window)
                                {
                                    window.focus(&down_state.read(cx).focus_handle.clone());
                                    let local_x = event.position.x - bounds.origin.x;
                                    let text_style = window.text_style();
                                    let font_size =
                                        text_style.font_size.to_pixels(window.rem_size());
                                    let shaped_click =
                                        shape(&down_content, &down_font, font_size, window);
                                    let idx = shaped_click.closest_index_for_x(local_x);
                                    let shift_held = event.modifiers.shift;
                                    down_state.update(cx, |s, cx| {
                                        s.is_selecting = true;
                                        if shift_held {
                                            s.select_to(idx, cx);
                                        } else {
                                            s.move_to(idx, cx);
                                        }
                                    });
                                }
                            }
                        });

                        window.on_mouse_event({
                            let move_state = move_state.clone();
                            let move_content = move_content.clone();
                            let move_font = move_font.clone();
                            move |event: &MouseMoveEvent, phase, window, cx| {
                                if phase == DispatchPhase::Bubble
                                    && move_state.read(cx).is_selecting
                                {
                                    let local_x = event.position.x - bounds.origin.x;
                                    let text_style = window.text_style();
                                    let font_size =
                                        text_style.font_size.to_pixels(window.rem_size());
                                    let shaped_move =
                                        shape(&move_content, &move_font, font_size, window);
                                    let idx = shaped_move.closest_index_for_x(local_x);
                                    move_state.update(cx, |s, cx| s.select_to(idx, cx));
                                }
                            }
                        });

                        window.on_mouse_event({
                            let up_state = up_state.clone();
                            move |_event: &MouseUpEvent, phase, _window, cx| {
                                if phase == DispatchPhase::Bubble {
                                    up_state.update(cx, |s, _cx| s.is_selecting = false);
                                }
                            }
                        });

                        bounds_cache_state.update(cx, |s, _cx| {
                            s.last_bounds = Some(bounds);
                        });
                    },
                )
                .size_full(),
            )
    }
}
