use std::io::{stdout, Write};

use crossterm::{
    cursor::MoveTo,
    event::{read, Event, KeyEvent},
    style::{PrintStyledContent, Stylize},
    QueueableCommand,
};

use crate::Editor;

pub fn get_command(e: &Editor) -> anyhow::Result<Option<String>> {
    let (fg, bg) = e.theme.default_colors();
    let width = e.width;

    let mut command = String::new();

    loop {
        stdout().queue(MoveTo(0, e.command_y() as u16))?;
        stdout().queue(PrintStyledContent(" ".repeat(width).with(fg).on(bg)))?;
        stdout().queue(MoveTo(0, e.command_y() as u16))?;
        stdout().queue(PrintStyledContent(format!(":{command}").with(fg).on(bg)))?;
        stdout().flush()?;

        match read()? {
            Event::Key(KeyEvent { code, .. }) => match code {
                crossterm::event::KeyCode::Esc => break,
                crossterm::event::KeyCode::Enter => break,
                crossterm::event::KeyCode::Backspace => {
                    command.pop();
                }
                crossterm::event::KeyCode::Char(c) => {
                    command.push(c);
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok(Some(command))
}
