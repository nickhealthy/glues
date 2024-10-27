use crate::{
    db::Db,
    state::notebook::{directory, note, InnerState, NotebookState},
    transition::{NormalModeTransition, NotebookTransition},
    Error, Event, KeyEvent, NotebookEvent, Result,
};

#[derive(Clone, Copy)]
pub enum VimState {
    Idle,
    Numbering(usize),
}

pub async fn consume(
    db: &mut Db,
    state: &mut NotebookState,
    vim_state: VimState,
    event: Event,
) -> Result<NotebookTransition> {
    match vim_state {
        VimState::Idle => consume_idle(db, state, event).await,
        VimState::Numbering(n) => consume_numbering(state, n, event).await,
    }
}

async fn consume_idle(
    db: &mut Db,
    state: &mut NotebookState,
    event: Event,
) -> Result<NotebookTransition> {
    use Event::*;
    use NotebookEvent::*;

    match event {
        Notebook(SelectNote(note)) => note::select(state, note),
        Notebook(SelectDirectory(directory)) => directory::select(state, directory),
        Notebook(UpdateNoteContent(content)) => note::update_content(db, state, content).await,
        Key(KeyEvent::E) | Notebook(EditNote) => note::edit(state).await,
        Key(KeyEvent::B) | Notebook(BrowseNoteTree) => note::browse(state).await,
        Key(KeyEvent::Num(n)) => {
            state.inner_state = InnerState::EditingNormalMode(VimState::Numbering(n.into()));

            Ok(NotebookTransition::EditingNormalMode(
                NormalModeTransition::NumberingMode,
            ))
        }
        Key(KeyEvent::J) => Ok(NotebookTransition::EditingNormalMode(
            NormalModeTransition::MoveCursorDown(1),
        )),
        Key(KeyEvent::K) => Ok(NotebookTransition::EditingNormalMode(
            NormalModeTransition::MoveCursorUp(1),
        )),
        Key(KeyEvent::H) => Ok(NotebookTransition::EditingNormalMode(
            NormalModeTransition::MoveCursorBack(1),
        )),
        Key(KeyEvent::L) => Ok(NotebookTransition::EditingNormalMode(
            NormalModeTransition::MoveCursorForward(1),
        )),
        event @ Key(_) => Ok(NotebookTransition::Inedible(event)),
        _ => Err(Error::Wip("todo: Notebook::consume".to_owned())),
    }
}

async fn consume_numbering(
    state: &mut NotebookState,
    n: usize,
    event: Event,
) -> Result<NotebookTransition> {
    use Event::*;

    match event {
        Key(KeyEvent::Num(n2)) => {
            state.inner_state = InnerState::EditingNormalMode(VimState::Numbering(n2 + n * 10));

            Ok(NotebookTransition::None)
        }
        Key(KeyEvent::J) => {
            state.inner_state = InnerState::EditingNormalMode(VimState::Idle);

            Ok(NotebookTransition::EditingNormalMode(
                NormalModeTransition::MoveCursorDown(n),
            ))
        }
        Key(KeyEvent::K) => {
            state.inner_state = InnerState::EditingNormalMode(VimState::Idle);

            Ok(NotebookTransition::EditingNormalMode(
                NormalModeTransition::MoveCursorUp(n),
            ))
        }
        Key(KeyEvent::H) => {
            state.inner_state = InnerState::EditingNormalMode(VimState::Idle);

            Ok(NotebookTransition::EditingNormalMode(
                NormalModeTransition::MoveCursorBack(n),
            ))
        }
        Key(KeyEvent::L) => {
            state.inner_state = InnerState::EditingNormalMode(VimState::Idle);

            Ok(NotebookTransition::EditingNormalMode(
                NormalModeTransition::MoveCursorForward(n),
            ))
        }
        Key(KeyEvent::Esc) => {
            state.inner_state = InnerState::EditingNormalMode(VimState::Idle);

            Ok(NotebookTransition::EditingNormalMode(
                NormalModeTransition::IdleMode,
            ))
        }
        event @ Key(_) => Ok(NotebookTransition::Inedible(event)),
        _ => Err(Error::Wip("todo: Notebook::consume".to_owned())),
    }
}