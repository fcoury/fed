use rustyline::{
    completion::Completer, highlight::Highlighter, hint::Hinter, history::FileHistory,
    validate::Validator, CompletionType, Config, Editor, Event, Helper, KeyCode, KeyEvent,
    Modifiers,
};

pub fn new_command_editor() -> anyhow::Result<Editor<CommandHelper, FileHistory>> {
    let mut rl = Editor::with_config(
        Config::builder()
            .completion_type(CompletionType::List)
            .auto_add_history(true)
            .edit_mode(rustyline::EditMode::Vi)
            .build(),
    )?;
    rl.bind_sequence(
        Event::KeySeq(vec![KeyEvent(KeyCode::Enter, Modifiers::NONE)]),
        rustyline::Cmd::AcceptLine,
    );

    Ok(rl)
}

pub struct CommandHelper;
impl Helper for CommandHelper {}
impl Completer for CommandHelper {
    type Candidate = String;
}
impl Hinter for CommandHelper {
    type Hint = String;
}
impl Highlighter for CommandHelper {}
impl Validator for CommandHelper {}
