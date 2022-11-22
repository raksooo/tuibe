use std::ops::Range;
use tui::layout::Rect;

pub fn list_range(area: Rect, items_length: usize, current_index: usize) -> Range<usize> {
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
