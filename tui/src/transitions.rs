use {
    super::{
        context::{self, notebook::TreeItem, ContextState},
        logger::*,
        App,
    },
    glues_core::{
        data::{Directory, Note},
        state::{
            notebook::{InnerState, VimNormalState},
            GetInner, NotebookState,
        },
        transition::{
            EntryTransition, NormalModeTransition, NotebookTransition, Transition,
            VisualModeTransition,
        },
        NotebookEvent,
    },
    std::time::SystemTime,
    tui_textarea::{CursorMove, TextArea},
};

impl App {
    pub(super) async fn handle_transition(&mut self, transition: Transition) {
        match transition {
            Transition::Entry(transition) => {
                self.handle_entry_transition(transition).await;
            }
            Transition::Notebook(transition) => {
                self.handle_notebook_transition(transition).await;
            }
            Transition::Log(message) => {
                log!("{message}");
                self.context.last_log = Some((message, SystemTime::now()));
            }
            Transition::Error(message) => {
                log!("[Err] {message}");
                self.context.alert = Some(message);
            }
        }
    }

    pub(super) async fn handle_entry_transition(&mut self, transition: EntryTransition) {
        match transition {
            EntryTransition::OpenNotebook => {
                log!("Opening notebook");

                let NotebookState { root, .. } = self.glues.state.get_inner().log_unwrap();
                self.context.state = ContextState::Notebook;
                self.context.notebook.update_items(root);
            }
            EntryTransition::Inedible(event) => {
                log!("Inedible event: {event}");
            }
            EntryTransition::None => {}
        }
    }

    pub(super) async fn handle_notebook_transition(&mut self, transition: NotebookTransition) {
        use context::notebook::ContextState;

        let NotebookState {
            root, inner_state, ..
        } = self.glues.state.get_inner().log_unwrap();
        let new_state = match inner_state {
            InnerState::NoteSelected | InnerState::DirectorySelected => {
                ContextState::NoteTreeBrowsing
            }
            InnerState::NoteTreeNumber(_) => ContextState::NoteTreeNumbering,
            InnerState::NoteMoreActions => ContextState::NoteActionsDialog,
            InnerState::DirectoryMoreActions => ContextState::DirectoryActionsDialog,
            InnerState::EditingNormalMode(VimNormalState::Idle) => {
                ContextState::EditorNormalMode { idle: true }
            }
            InnerState::EditingNormalMode(_) => ContextState::EditorNormalMode { idle: false },
            InnerState::EditingVisualMode(_) => ContextState::EditorVisualMode,
            InnerState::EditingInsertMode => ContextState::EditorInsertMode,
        };

        if self.context.notebook.state != new_state {
            self.context.notebook.state = new_state;
        }

        match transition {
            NotebookTransition::ShowVimKeymap(kind) => {
                self.context.vim_keymap = Some(kind);
            }
            NotebookTransition::OpenDirectory { id, .. } => {
                log!("Opening directory {id}");
                self.context.notebook.update_items(root);
            }
            NotebookTransition::CloseDirectory(id) => {
                log!("Closing directory {id}");
                self.context.notebook.update_items(root);
                self.context.notebook.select_item(&id);
            }
            NotebookTransition::OpenNote { note, content } => {
                self.context.notebook.open_note(note, content);
            }
            NotebookTransition::ViewMode(_note) => {
                self.save().await;
            }
            NotebookTransition::BrowseNoteTree => {}
            NotebookTransition::RemoveNote {
                selected_directory, ..
            }
            | NotebookTransition::RemoveDirectory {
                selected_directory, ..
            } => {
                self.context.notebook.select_item(&selected_directory.id);
                self.context.notebook.update_items(root);
            }
            NotebookTransition::RenameNote(_) | NotebookTransition::RenameDirectory(_) => {
                self.context.notebook.update_items(root);
            }
            NotebookTransition::AddNote(Note {
                id,
                directory_id: parent_id,
                ..
            })
            | NotebookTransition::AddDirectory(Directory { id, parent_id, .. }) => {
                self.glues
                    .dispatch(NotebookEvent::OpenDirectory(parent_id.clone()).into())
                    .await
                    .log_unwrap();
                let NotebookState { root, .. } = self.glues.state.get_inner().log_unwrap();

                self.context.notebook.update_items(root);
                self.context.notebook.select_item(&id);
            }
            NotebookTransition::SelectNext(n) => {
                self.context.notebook.select_next(n);

                let event = match self.context.notebook.selected() {
                    TreeItem::Directory { value, .. } => {
                        NotebookEvent::SelectDirectory(value.clone()).into()
                    }
                    TreeItem::Note { value, .. } => NotebookEvent::SelectNote(value.clone()).into(),
                };

                self.glues.dispatch(event).await.log_unwrap();
            }
            NotebookTransition::SelectPrev(n) => {
                self.context.notebook.select_prev(n);

                let event = match self.context.notebook.selected() {
                    TreeItem::Directory { value, .. } => {
                        NotebookEvent::SelectDirectory(value.clone()).into()
                    }
                    TreeItem::Note { value, .. } => NotebookEvent::SelectNote(value.clone()).into(),
                };

                self.glues.dispatch(event).await.log_unwrap();
            }
            NotebookTransition::EditingNormalMode(transition) => {
                self.handle_normal_mode_transition(transition);
            }
            NotebookTransition::EditingVisualMode(transition) => {
                self.handle_visual_mode_transition(transition);
            }
            NotebookTransition::Alert(message) => {
                log!("[Alert] {message}");
                self.context.alert = Some(message);
            }
            _ => {}
        }
    }

    fn handle_normal_mode_transition(&mut self, transition: NormalModeTransition) {
        use NormalModeTransition::*;

        match transition {
            IdleMode => {
                self.context.notebook.editor.cancel_selection();
            }
            ToggleMode | NumberingMode | GatewayMode | YankMode | DeleteMode | DeleteInsideMode
            | ChangeMode | ChangeInsideMode => {}
            ToggleLineNumbers => {
                self.context.notebook.show_line_number = !self.context.notebook.show_line_number;
            }
            ToggleBrowser => {
                self.context.notebook.show_browser = !self.context.notebook.show_browser;
            }
            MoveCursorDown(n) => {
                let editor = &mut self.context.notebook.editor;
                let cursor_move = cursor_move_down(editor, n);

                editor.move_cursor(cursor_move);
            }
            MoveCursorUp(n) => {
                let editor = &mut self.context.notebook.editor;
                let cursor_move = cursor_move_up(editor, n);

                editor.move_cursor(cursor_move);
            }
            MoveCursorBack(n) => {
                let (row, col) = self.context.notebook.editor.cursor();
                let cursor_move = if col < n {
                    CursorMove::Head
                } else {
                    CursorMove::Jump(row as u16, (col - n) as u16)
                };

                self.context.notebook.editor.move_cursor(cursor_move);
            }
            MoveCursorForward(n) => {
                let editor = &mut self.context.notebook.editor;
                let cursor_move = cursor_move_forward(editor, n);

                editor.move_cursor(cursor_move);
            }
            MoveCursorWordForward(n) => {
                for _ in 0..n {
                    self.context
                        .notebook
                        .editor
                        .move_cursor(CursorMove::WordForward);
                }
            }
            MoveCursorWordEnd(n) => {
                move_cursor_word_end(&mut self.context.notebook.editor, n);
            }
            MoveCursorWordBack(n) => {
                move_cursor_word_back(&mut self.context.notebook.editor, n);
            }
            MoveCursorLineStart => {
                self.context.notebook.editor.move_cursor(CursorMove::Head);
            }
            MoveCursorLineEnd => {
                self.context.notebook.editor.move_cursor(CursorMove::End);
            }
            MoveCursorLineNonEmptyStart => {
                move_cursor_to_line_non_empty_start(&mut self.context.notebook.editor);
            }
            MoveCursorTop => {
                self.context.notebook.editor.move_cursor(CursorMove::Top);
            }
            MoveCursorBottom => {
                self.context.notebook.editor.move_cursor(CursorMove::Bottom);
            }
            MoveCursorToLine(n) => {
                self.context
                    .notebook
                    .editor
                    .move_cursor(CursorMove::Jump((n - 1) as u16, 0));
                self.context
                    .notebook
                    .editor
                    .move_cursor(CursorMove::WordForward);
            }
            InsertNewLineBelow => {
                self.context.notebook.editor.move_cursor(CursorMove::End);
                self.context.notebook.editor.insert_newline();
            }
            InsertNewLineAbove => {
                self.context.notebook.editor.move_cursor(CursorMove::Head);
                self.context.notebook.editor.insert_newline();
                self.context.notebook.editor.move_cursor(CursorMove::Up);
            }
            InsertAtCursor => {}
            InsertAtLineStart => {
                self.context.notebook.editor.move_cursor(CursorMove::Head);
            }
            InsertAfterCursor => {
                self.context
                    .notebook
                    .editor
                    .move_cursor(CursorMove::Forward);
            }
            InsertAtLineEnd => {
                self.context.notebook.editor.move_cursor(CursorMove::End);
            }
            DeleteChars(n) => {
                let editor = &mut self.context.notebook.editor;
                editor.start_selection();
                let cursor_move = cursor_move_forward(editor, n);

                editor.move_cursor(cursor_move);
                editor.cut();
            }
            Paste => {
                let editor = &mut self.context.notebook.editor;
                if self.context.notebook.line_yanked {
                    editor.move_cursor(CursorMove::End);
                    editor.insert_newline();
                    editor.paste();
                    move_cursor_to_line_non_empty_start(editor);
                } else {
                    editor.paste();
                }
            }
            Undo => {
                self.context.notebook.editor.undo();
            }
            Redo => {
                self.context.notebook.editor.redo();
            }
            YankLines(n) => {
                let editor = &mut self.context.notebook.editor;
                let cursor = editor.cursor();
                editor.move_cursor(CursorMove::Head);
                editor.start_selection();
                let cursor_move = cursor_move_down(editor, n - 1);
                editor.move_cursor(cursor_move);
                editor.move_cursor(CursorMove::End);
                editor.copy();
                editor.cancel_selection();
                editor.move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
                self.context.notebook.line_yanked = true;
            }
            DeleteLines(n) => {
                let editor = &mut self.context.notebook.editor;
                let (row, _) = editor.cursor();

                editor.move_cursor(CursorMove::Head);
                editor.start_selection();
                let cursor_move = cursor_move_down(editor, n - 1);
                editor.move_cursor(cursor_move);
                editor.move_cursor(CursorMove::End);
                editor.cut();

                if row == 0 {
                    editor.move_cursor(CursorMove::Down);
                    editor.move_cursor(CursorMove::Head);
                    editor.delete_char();
                } else {
                    editor.delete_char();
                    editor.move_cursor(CursorMove::Down);
                }

                move_cursor_to_line_non_empty_start(editor);
                self.context.notebook.line_yanked = true;
            }
            DeleteLinesAndInsert(n) => {
                let editor = &mut self.context.notebook.editor;
                editor.move_cursor(CursorMove::Head);
                editor.start_selection();
                let cursor_move = cursor_move_down(editor, n - 1);
                editor.move_cursor(cursor_move);
                editor.move_cursor(CursorMove::End);
                editor.cut();
                self.context.notebook.line_yanked = true;
            }
            DeleteInsideWord(n) => {
                let editor = &mut self.context.notebook.editor;
                let cursor = editor.cursor();
                editor.move_cursor(CursorMove::WordBack);
                editor.move_cursor(CursorMove::WordEnd);
                let cursor_be = editor.cursor();
                editor.move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));

                if cursor_be >= cursor {
                    editor.move_cursor(CursorMove::WordBack);
                }

                editor.start_selection();
                move_cursor_word_end(editor, n);
                editor.move_cursor(CursorMove::Forward);
                editor.cut();

                self.context.notebook.line_yanked = false;
            }
            DeleteWordEnd(n) => {
                let editor = &mut self.context.notebook.editor;
                editor.start_selection();
                move_cursor_word_end(editor, n);
                editor.move_cursor(CursorMove::Forward);
                editor.cut();

                self.context.notebook.line_yanked = false;
            }
            DeleteWordBack(n) => {
                let editor = &mut self.context.notebook.editor;
                editor.start_selection();
                move_cursor_word_back(editor, n);
                editor.cut();

                self.context.notebook.line_yanked = false;
            }
            DeleteLineStart => {
                let editor = &mut self.context.notebook.editor;
                editor.start_selection();
                editor.move_cursor(CursorMove::Head);
                editor.cut();

                self.context.notebook.line_yanked = false;
            }
            DeleteLineEnd(n) => {
                let editor = &mut self.context.notebook.editor;
                editor.start_selection();
                let cursor_move = cursor_move_down(editor, n - 1);
                editor.move_cursor(cursor_move);
                editor.move_cursor(CursorMove::End);
                editor.cut();

                self.context.notebook.line_yanked = false;
            }
        };
    }

    fn handle_visual_mode_transition(&mut self, transition: VisualModeTransition) {
        use VisualModeTransition::*;

        match transition {
            IdleMode => {
                self.context.notebook.editor.start_selection();
            }
            NumberingMode | GatewayMode => {}
            MoveCursorDown(n) => {
                let editor = &mut self.context.notebook.editor;
                let cursor_move = cursor_move_down(editor, n);

                editor.move_cursor(cursor_move);
            }
            MoveCursorUp(n) => {
                let editor = &mut self.context.notebook.editor;
                let cursor_move = cursor_move_up(editor, n);

                editor.move_cursor(cursor_move);
            }
            MoveCursorBack(n) => {
                let (row, col) = self.context.notebook.editor.cursor();
                let cursor_move = if col < n {
                    CursorMove::Head
                } else {
                    CursorMove::Jump(row as u16, (col - n) as u16)
                };

                self.context.notebook.editor.move_cursor(cursor_move);
            }
            MoveCursorForward(n) => {
                let editor = &mut self.context.notebook.editor;
                let cursor_move = cursor_move_forward(editor, n);

                editor.move_cursor(cursor_move);
            }
            MoveCursorWordForward(n) => {
                for _ in 0..n {
                    self.context
                        .notebook
                        .editor
                        .move_cursor(CursorMove::WordForward);
                }
            }
            MoveCursorWordEnd(n) => {
                for _ in 0..n {
                    self.context
                        .notebook
                        .editor
                        .move_cursor(CursorMove::WordEnd);
                }
            }
            MoveCursorWordBack(n) => {
                for _ in 0..n {
                    self.context
                        .notebook
                        .editor
                        .move_cursor(CursorMove::WordBack);
                }
            }
            MoveCursorLineStart => {
                self.context.notebook.editor.move_cursor(CursorMove::Head);
            }
            MoveCursorLineEnd => {
                self.context.notebook.editor.move_cursor(CursorMove::End);
            }
            MoveCursorLineNonEmptyStart => {
                move_cursor_to_line_non_empty_start(&mut self.context.notebook.editor);
            }
            MoveCursorBottom => {
                self.context.notebook.editor.move_cursor(CursorMove::Bottom);
            }
            MoveCursorTop => {
                self.context.notebook.editor.move_cursor(CursorMove::Top);
            }
            MoveCursorToLine(n) => {
                self.context
                    .notebook
                    .editor
                    .move_cursor(CursorMove::Jump((n - 1) as u16, 0));
                self.context
                    .notebook
                    .editor
                    .move_cursor(CursorMove::WordForward);
            }
            YankSelection => {
                let editor = &mut self.context.notebook.editor;
                reselect_for_yank(editor);
                editor.copy();
                self.context.notebook.line_yanked = false;
            }
            DeleteSelection => {
                let editor = &mut self.context.notebook.editor;
                reselect_for_yank(editor);
                editor.cut();
                self.context.notebook.line_yanked = false;
            }
            DeleteSelectionAndInsertMode => {
                let editor = &mut self.context.notebook.editor;
                reselect_for_yank(editor);
                self.context.notebook.editor.cut();
                self.context.notebook.line_yanked = false;
            }
        }
    }

    pub(crate) async fn save(&mut self) {
        let content = self.context.notebook.editor.lines().join("\n");
        let event = NotebookEvent::UpdateNoteContent(content).into();

        self.glues.dispatch(event).await.log_unwrap();
    }
}

fn cursor_move_forward(editor: &TextArea, n: usize) -> CursorMove {
    let (row, col) = editor.cursor();
    if col + n >= editor.lines()[row].len() {
        CursorMove::End
    } else {
        CursorMove::Jump(row as u16, (col + n) as u16)
    }
}

fn cursor_move_down(editor: &TextArea, n: usize) -> CursorMove {
    let num_lines = editor.lines().len();
    let (row, col) = editor.cursor();
    if row + n >= num_lines {
        CursorMove::Bottom
    } else {
        CursorMove::Jump((row + n) as u16, col as u16)
    }
}

fn cursor_move_up(editor: &TextArea, n: usize) -> CursorMove {
    let (row, col) = editor.cursor();
    if row < n {
        CursorMove::Top
    } else {
        CursorMove::Jump((row - n) as u16, col as u16)
    }
}

fn move_cursor_to_line_non_empty_start(editor: &mut TextArea) {
    editor.move_cursor(CursorMove::Head);

    let (row, _) = editor.cursor();
    let is_whitespace_at_first = editor.lines()[row]
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false);
    if is_whitespace_at_first {
        editor.move_cursor(CursorMove::WordForward);
    }
}

fn move_cursor_word_end(editor: &mut TextArea, n: usize) {
    for _ in 0..n {
        editor.move_cursor(CursorMove::WordEnd);
    }
}

fn move_cursor_word_back(editor: &mut TextArea, n: usize) {
    for _ in 0..n {
        editor.move_cursor(CursorMove::WordBack);
    }
}

fn reselect_for_yank(editor: &mut TextArea) {
    let (begin, end) = match editor.selection_range() {
        None => return,
        Some(range) => range,
    };

    editor.cancel_selection();
    editor.move_cursor(CursorMove::Jump(begin.0 as u16, begin.1 as u16));
    editor.start_selection();
    editor.move_cursor(CursorMove::Jump(end.0 as u16, end.1 as u16));
    editor.move_cursor(CursorMove::Forward);
}
