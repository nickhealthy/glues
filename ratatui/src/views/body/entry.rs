use {
    crate::context::EntryContext,
    glues_core::state::EntryState,
    ratatui::{
        layout::{Alignment, Constraint::Length, Flex, Layout, Rect},
        style::{Color, Style},
        widgets::{Block, HighlightSpacing, List, ListDirection, Padding},
        Frame,
    },
};

pub fn draw(frame: &mut Frame, area: Rect, _state: &EntryState, context: &mut EntryContext) {
    let [area] = Layout::horizontal([Length(24)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Length(9)]).flex(Flex::Center).areas(area);

    let block = Block::bordered()
        .padding(Padding::new(2, 2, 1, 1))
        .title("[Glues] Open Notes")
        .title_alignment(Alignment::Center);

    let items = ["Instant", "CSV", "JSON", "File", "Git"];
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().fg(Color::White).bg(Color::DarkGray))
        .highlight_symbol(" ")
        .highlight_spacing(HighlightSpacing::Always)
        .direction(ListDirection::TopToBottom);

    frame.render_stateful_widget(list, area, &mut context.list_state);
}
