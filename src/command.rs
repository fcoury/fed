use rustyline::{
    history::FileHistory, Completer, CompletionType, Config, Editor, Event, Helper, Highlighter,
    Hinter, KeyCode, KeyEvent, Modifiers, Validator,
};

pub fn new_command_editor() -> anyhow::Result<Editor<MaskingHighlighter, FileHistory>> {
    let mut rl = Editor::with_config(
        Config::builder()
            .completion_type(CompletionType::List)
            .auto_add_history(true)
            .edit_mode(rustyline::EditMode::Vi)
            .newline(false)
            .build(),
    )?;
    rl.bind_sequence(
        Event::KeySeq(vec![KeyEvent(KeyCode::Enter, Modifiers::NONE)]),
        rustyline::Cmd::AcceptLine,
    );

    Ok(rl)
}

#[derive(Completer, Helper, Hinter, Validator, Highlighter)]
pub struct MaskingHighlighter;
