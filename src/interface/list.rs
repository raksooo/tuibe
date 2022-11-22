use std::ops::Range;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::ListItem,
};

pub fn generate_items<T>(
    area: Rect,
    current_index: usize,
    items: Vec<T>,
    f: impl Fn(T) -> String,
) -> Vec<ListItem<'static>> {
    let range = list_range(area, items.len(), current_index);
    items
        .into_iter()
        .enumerate()
        .skip(range.start)
        .take(range.end - range.start)
        .map(|(i, item)| {
            let item = ListItem::new(f(item));
            if i == current_index {
                item.style(Style::default().fg(Color::Green))
            } else {
                item
            }
        })
        .collect()
}

fn list_range(area: Rect, items_length: usize, current_index: usize) -> Range<usize> {
    let height: usize = area.height.into();

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
