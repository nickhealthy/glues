use {
    super::render_directory,
    crate::{traits::*, views::note_tree::note::render_note, Node},
    cursive::{
        view::Nameable,
        views::{LinearLayout, PaddedView},
        Cursive, View,
    },
    glues_core::types::DirectoryId,
};

pub fn render_item_list(siv: &mut Cursive, directory_id: DirectoryId) -> impl View {
    let directories = siv
        .glues()
        .fetch_directories(directory_id.clone())
        .log_unwrap();
    let notes = siv.glues().fetch_notes(directory_id.clone()).log_unwrap();
    let mut layout = LinearLayout::vertical();

    for child in directories {
        layout.add_child(render_directory(siv, child));
    }

    for child in notes {
        layout.add_child(render_note(child));
    }

    let layout = layout.with_name(
        Node::note_tree()
            .directory(&directory_id)
            .note_list()
            .name(),
    );

    PaddedView::lrtb(1, 0, 0, 0, layout)
}