use sorted_vec::SortedSet;
use std::ops::Range;
use tui::{
    style::{Color, Style},
    widgets::{List as ListWidget, ListItem},
};

pub trait Same {
    fn same(&self, other: &Self) -> bool;
}

pub struct List<T: Clone + Ord + Same + Into<ListItem<'static>>> {
    items: SortedSet<T>,
    current_index: Option<usize>,
}

impl<T: Clone + Ord + Same + Into<ListItem<'static>>> List<T> {
    pub fn new() -> Self {
        Self {
            items: SortedSet::new(),
            current_index: None,
        }
    }

    pub fn add(&mut self, item: T) {
        self.current_index = self.current_index.or(Some(0));
        self.items.mutate_vec(|items| items.push(item));
    }

    pub fn remove(&mut self, item_to_remove: &T) {
        self.items
            .mutate_vec(|items| items.retain(|item| !item.same(item_to_remove)));
        self.current_index = self.items.first().map(|_| 0);
    }

    pub fn clear(&mut self) {
        self.current_index = None;
        self.items.clear();
    }

    pub fn move_up(&mut self) {
        self.mutate_current_index(|current_index| current_index.saturating_sub(1));
    }

    pub fn move_down(&mut self) {
        self.mutate_current_index(|current_index| current_index.saturating_add(1));
    }

    pub fn move_top(&mut self) {
        self.mutate_current_index(|_| 0);
    }

    pub fn move_bottom(&mut self) {
        self.mutate_current_index(|_| usize::MAX);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub fn get_current_item(&self) -> Option<T> {
        let Some(current_index) = self.current_index else { return None };
        self.items.get(current_index).cloned()
    }

    pub fn mutate_every_item(&mut self, f: impl Fn(&mut T)) {
        self.items.mutate_vec(|items| items.iter_mut().for_each(f));
    }

    pub fn mutate_current_item(&mut self, f: impl FnOnce(&mut T)) {
        let Some(current_index) = self.current_index else { return };
        self.items
            .mutate_vec(|items| items.get_mut(current_index).map(f));
    }

    pub fn list(&self, height: usize) -> ListWidget<'_> {
        self.map_list(height, |item| item)
    }

    pub fn map_list<F, R>(&self, height: usize, f: F) -> ListWidget<'_>
    where
        R: Into<ListItem<'static>>,
        F: Fn(T) -> R,
    {
        let items = self.map_visible_items(height, f);
        ListWidget::new(items)
    }

    fn map_visible_items<R, F>(&self, height: usize, f: F) -> Vec<ListItem<'_>>
    where
        R: Into<ListItem<'static>>,
        F: Fn(T) -> R,
    {
        let Some(current_index) = self.current_index else { return Default::default() };
        let range = Self::list_range(height, self.items.len(), current_index);
        self.items
            .iter()
            .cloned()
            .enumerate()
            .skip(range.start)
            .take(range.len())
            .map(|(i, item)| {
                let item: ListItem<'_> = f(item).into();
                if i == current_index {
                    item.style(Style::default().fg(Color::Green))
                } else {
                    item
                }
            })
            .collect()
    }

    fn list_range(height: usize, items_length: usize, current_index: usize) -> Range<usize> {
        if height >= items_length || current_index < height / 2 {
            0..height
        } else if current_index >= items_length - height / 2 {
            let start_index = items_length - height + 1;
            let end_index = start_index + height;
            start_index..end_index
        } else {
            let start_index = current_index - (height / 2);
            let end_index = start_index + height;
            start_index..end_index
        }
    }

    // Mutates the index and clamps it to the available item indexes
    fn mutate_current_index(&mut self, f: impl Fn(usize) -> usize) {
        self.current_index = self
            .current_index
            .map(|current_index| f(current_index).clamp(0, self.items.len() - 1));
    }
}
