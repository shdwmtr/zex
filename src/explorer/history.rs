use std::path::{Path, PathBuf};

pub struct History {
    entries: Vec<PathBuf>,
    index: usize,
}

impl History {
    pub fn new(start: PathBuf) -> Self {
        Self {
            entries: vec![start],
            index: 0,
        }
    }

    pub fn current(&self) -> &Path {
        &self.entries[self.index]
    }

    pub fn can_go_back(&self) -> bool {
        self.index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    pub fn navigate(&mut self, path: PathBuf) -> bool {
        if path == *self.current() {
            return false;
        }
        self.entries.truncate(self.index + 1);
        self.entries.push(path);
        self.index = self.entries.len() - 1;
        true
    }

    pub fn back(&mut self) -> bool {
        if self.can_go_back() {
            self.index -= 1;
            true
        } else {
            false
        }
    }

    pub fn forward(&mut self) -> bool {
        if self.can_go_forward() {
            self.index += 1;
            true
        } else {
            false
        }
    }

    pub fn back_entries(&self) -> impl Iterator<Item = (usize, &Path)> {
        self.entries[..self.index]
            .iter()
            .enumerate()
            .rev()
            .map(|(ix, path)| (ix, path.as_path()))
    }

    pub fn forward_entries(&self) -> impl Iterator<Item = (usize, &Path)> {
        self.entries[self.index + 1..]
            .iter()
            .enumerate()
            .map(|(offset, path)| (self.index + 1 + offset, path.as_path()))
    }

    pub fn jump_to(&mut self, index: usize) -> bool {
        if index == self.index || index >= self.entries.len() {
            return false;
        }
        self.index = index;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_the_initial_path_with_no_back_or_forward() {
        let history = History::new(PathBuf::from("/home"));

        assert_eq!(history.current(), Path::new("/home"));
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn navigate_to_the_same_path_is_a_no_op() {
        let mut history = History::new(PathBuf::from("/a"));

        assert!(!history.navigate(PathBuf::from("/a")));
        assert!(!history.can_go_back());
    }

    #[test]
    fn back_and_forward_move_the_cursor() {
        let mut history = History::new(PathBuf::from("/a"));
        history.navigate(PathBuf::from("/b"));
        history.navigate(PathBuf::from("/c"));

        assert!(history.back());
        assert_eq!(history.current(), Path::new("/b"));
        assert!(history.can_go_forward());

        assert!(history.back());
        assert_eq!(history.current(), Path::new("/a"));
        assert!(!history.can_go_back());

        assert!(history.forward());
        assert_eq!(history.current(), Path::new("/b"));

        assert!(history.forward());
        assert_eq!(history.current(), Path::new("/c"));
        assert!(!history.can_go_forward());
    }

    #[test]
    fn back_at_the_start_and_forward_at_the_end_are_no_ops() {
        let mut history = History::new(PathBuf::from("/a"));

        assert!(!history.back());
        assert_eq!(history.current(), Path::new("/a"));
        assert!(!history.forward());
        assert_eq!(history.current(), Path::new("/a"));
    }

    #[test]
    fn navigating_from_a_rewound_cursor_discards_forward_history() {
        let mut history = History::new(PathBuf::from("/a"));
        history.navigate(PathBuf::from("/b"));
        history.navigate(PathBuf::from("/c"));
        history.back();
        history.back();
        assert_eq!(history.current(), Path::new("/a"));

        history.navigate(PathBuf::from("/d"));

        assert_eq!(history.current(), Path::new("/d"));
        assert!(!history.can_go_forward());
        assert!(history.back());
        assert_eq!(history.current(), Path::new("/a"));
    }

    #[test]
    fn back_and_forward_entries_list_the_rest_of_the_stack() {
        let mut history = History::new(PathBuf::from("/a"));
        history.navigate(PathBuf::from("/b"));
        history.navigate(PathBuf::from("/c"));
        history.back();

        let back: Vec<_> = history
            .back_entries()
            .map(|(ix, path)| (ix, path.to_path_buf()))
            .collect();
        assert_eq!(back, vec![(0, PathBuf::from("/a"))]);

        let forward: Vec<_> = history
            .forward_entries()
            .map(|(ix, path)| (ix, path.to_path_buf()))
            .collect();
        assert_eq!(forward, vec![(2, PathBuf::from("/c"))]);
    }

    #[test]
    fn jump_to_moves_directly_to_an_index() {
        let mut history = History::new(PathBuf::from("/a"));
        history.navigate(PathBuf::from("/b"));
        history.navigate(PathBuf::from("/c"));

        assert!(history.jump_to(0));
        assert_eq!(history.current(), Path::new("/a"));
        assert!(history.can_go_forward());

        assert!(!history.jump_to(0));
        assert!(!history.jump_to(10));
    }
}
