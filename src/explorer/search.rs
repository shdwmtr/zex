use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use gpui::{AppContext, Context, Entity, Subscription, Task, UniformListScrollHandle, Window};

use crate::search::{self, ContentMatch, NameMatch, SearchOptions, SearchScope};
use crate::settings::CaseSensitivity;
use crate::ui::text_input::{TextInputEvent as InputEvent, TextInputState as InputState};

use super::Explorer;

const SEARCH_DEBOUNCE_MS: u64 = 150;

pub enum SearchResults {
    Contents(Vec<ContentMatch>),
    Names(Vec<NameMatch>),
}

impl SearchResults {
    pub fn len(&self) -> usize {
        match self {
            SearchResults::Contents(items) => items.len(),
            SearchResults::Names(items) => items.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SearchStatus {
    Idle,
    Searching,
    Done,
    Error,
}

pub struct SearchState {
    pub root: PathBuf,
    pub origin: Entity<Explorer>,
    pub input: Entity<InputState>,
    pub scope: SearchScope,
    pub case: CaseSensitivity,
    pub regex: bool,
    pub whole_word: bool,
    pub include_hidden: bool,
    pub respect_gitignore: bool,
    pub results: SearchResults,
    pub truncated: bool,
    pub selected_index: Option<usize>,
    pub status: SearchStatus,
    pub error: Option<String>,
    pub scroll_handle: UniformListScrollHandle,
    search_task: Option<Task<()>>,
    active_cancel: Arc<AtomicBool>,
    _input_subscription: Subscription,
}

impl Explorer {
    pub fn open_search_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_trash() || self.disk_usage.is_some() {
            return;
        }
        let Some(pane) = self.pane.clone() else { return };
        let dir = self.current_dir().to_path_buf();
        let show_hidden = self.show_hidden;
        let origin = cx.entity();
        pane.update(cx, |pane, cx| pane.spawn_search_tab(dir, show_hidden, origin, window, cx));
    }

    pub fn begin_search(&mut self, origin: Entity<Explorer>, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_trash() || self.disk_usage.is_some() {
            return;
        }
        if let Some(state) = &self.search {
            let input = state.input.clone();
            input.update(cx, |input, cx| input.focus(window, cx));
            return;
        }

        self.watcher = None;
        self.watch_task = None;
        self.git_task = None;
        self.git_poll_task = None;
        self.free_space_task = None;

        let settings = self.search_settings.clone();
        let root = self.current_dir().to_path_buf();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search this directory"));
        input.update(cx, |input, cx| input.focus(window, cx));

        let input_subscription = cx.subscribe(&input, |explorer: &mut Self, _input, event, cx| {
            if let InputEvent::Changed { .. } = event {
                explorer.requery_search(cx);
            }
        });

        self.search = Some(SearchState {
            root,
            origin,
            input,
            scope: SearchScope::Names,
            case: settings.default_case,
            regex: false,
            whole_word: false,
            include_hidden: settings.include_hidden,
            respect_gitignore: settings.respect_gitignore,
            results: SearchResults::Names(Vec::new()),
            truncated: false,
            selected_index: None,
            status: SearchStatus::Idle,
            error: None,
            scroll_handle: UniformListScrollHandle::new(),
            search_task: None,
            active_cancel: Arc::new(AtomicBool::new(false)),
            _input_subscription: input_subscription,
        });
        cx.notify();
    }

    pub fn cancel_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.search.take() else { return };
        state.active_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        cx.notify();
        self.activate_and_close_for(state.origin, window, cx);
    }

    fn activate_and_close_for(&mut self, origin: Entity<Explorer>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pane) = self.pane.clone() else {
            window.focus(&self.focus_handle);
            return;
        };
        let self_id = cx.entity().entity_id();
        let origin_id = origin.entity_id();
        let origin_focus_handle = origin.read(cx).focus_handle.clone();

        pane.update(cx, |pane, cx| {
            if let Some(self_ix) = pane.tabs.iter().position(|tab| tab.entity_id() == self_id) {
                pane.close_tab(self_ix, cx);
            }
            if let Some(origin_ix) = pane.tabs.iter().position(|tab| tab.entity_id() == origin_id) {
                pane.active_index = origin_ix;
                cx.notify();
            }
        });

        window.focus(&origin_focus_handle);
    }

    pub fn set_search_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        if state.scope == scope {
            return;
        }
        state.scope = scope;
        self.requery_search(cx);
    }

    pub fn cycle_search_case(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        state.case = match state.case {
            CaseSensitivity::Sensitive => CaseSensitivity::Insensitive,
            CaseSensitivity::Insensitive => CaseSensitivity::Smart,
            CaseSensitivity::Smart => CaseSensitivity::Sensitive,
        };
        self.requery_search(cx);
    }

    pub fn toggle_search_regex(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        state.regex = !state.regex;
        self.requery_search(cx);
    }

    pub fn toggle_search_whole_word(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        state.whole_word = !state.whole_word;
        self.requery_search(cx);
    }

    pub fn toggle_search_hidden(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        state.include_hidden = !state.include_hidden;
        self.requery_search(cx);
    }

    pub fn toggle_search_gitignore(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        state.respect_gitignore = !state.respect_gitignore;
        self.requery_search(cx);
    }

    pub fn select_next_result(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        let len = state.results.len();
        if len == 0 {
            return;
        }
        state.selected_index = Some(match state.selected_index {
            Some(ix) => (ix + 1) % len,
            None => 0,
        });
        cx.notify();
    }

    pub fn select_prev_result(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        let len = state.results.len();
        if len == 0 {
            return;
        }
        state.selected_index = Some(match state.selected_index {
            Some(0) => len - 1,
            Some(ix) => ix - 1,
            None => len - 1,
        });
        cx.notify();
    }

    pub fn select_result(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        if index < state.results.len() {
            state.selected_index = Some(index);
            cx.notify();
        }
    }

    pub fn reveal_selected_result(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = &self.search else { return };
        let Some(ix) = state.selected_index else { return };
        let path = match &state.results {
            SearchResults::Contents(items) => items.get(ix).map(|item| item.path.clone()),
            SearchResults::Names(items) => items.get(ix).map(|item| item.path.clone()),
        };
        let Some(path) = path else { return };
        self.reveal_path(path, window, cx);
    }

    pub fn reveal_result(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.select_result(index, cx);
        self.reveal_selected_result(window, cx);
    }

    pub fn reveal_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.search.take() else { return };
        state.active_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        let origin = state.origin;

        origin.update(cx, |origin_explorer, cx| {
            if let Some(parent) = path.parent().map(Path::to_path_buf) {
                let already_there = parent.as_path() == origin_explorer.current_dir();
                origin_explorer.navigate_to(parent, cx);
                if already_there {
                    origin_explorer.enter_directory(cx);
                }
            }
            origin_explorer.selected = std::iter::once(path.clone()).collect();
            origin_explorer.focused_path = Some(path);
            cx.notify();
        });

        self.activate_and_close_for(origin, window, cx);
    }

    pub fn requery_search(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.search else { return };
        state.active_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        state.search_task = None;
        state.status = SearchStatus::Searching;
        state.error = None;

        let query = state.input.read(cx).value().to_string();
        let dir = state.root.clone();
        let scope = state.scope;
        let options = SearchOptions {
            query,
            case: state.case,
            regex: state.regex,
            whole_word: state.whole_word,
            include_hidden: state.include_hidden,
            respect_gitignore: state.respect_gitignore,
        };
        let cli = self.search_settings.cli.clone();
        let max_results = self.search_settings.max_results as usize;
        let cancel = Arc::new(AtomicBool::new(false));
        state.active_cancel = cancel.clone();

        state.search_task = Some(cx.spawn(async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(SEARCH_DEBOUNCE_MS))
                .await;

            let outcome = cx
                .background_executor()
                .spawn(async move {
                    match scope {
                        SearchScope::Contents => search::run_content_search(&dir, &options, &cli, max_results, &cancel)
                            .map(|outcome| (SearchResults::Contents(outcome.items), outcome.truncated)),
                        SearchScope::Names => search::run_name_search(&dir, &options, &cli, max_results, &cancel)
                            .map(|outcome| (SearchResults::Names(outcome.items), outcome.truncated)),
                    }
                })
                .await;

            let _ = weak.update(cx, |explorer, cx| {
                let Some(state) = &mut explorer.search else { return };
                match outcome {
                    Ok((results, truncated)) => {
                        state.selected_index = if results.is_empty() { None } else { Some(0) };
                        state.results = results;
                        state.truncated = truncated;
                        state.status = SearchStatus::Done;
                        state.error = None;
                    }
                    Err(err) => {
                        state.results = match state.scope {
                            SearchScope::Contents => SearchResults::Contents(Vec::new()),
                            SearchScope::Names => SearchResults::Names(Vec::new()),
                        };
                        state.truncated = false;
                        state.selected_index = None;
                        state.status = SearchStatus::Error;
                        state.error = Some(err);
                    }
                }
                cx.notify();
            });
        }));
    }
}
