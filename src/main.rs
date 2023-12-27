use std::io::{stdout, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    style::Print,
    terminal::{self, ClearType},
    QueueableCommand,
};

enum Mode {
    Normal,
    Insert,
}

struct Editor {
    mode: Mode,
    buffer: Vec<String>,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
}

impl Editor {
    pub fn new() -> anyhow::Result<Self> {
        terminal::enable_raw_mode()?;
        let (width, height) = terminal::size()?;

        Ok(Self {
            mode: Mode::Normal,
            buffer: vec![],
            width: width as usize,
            height: height as usize,
            x: 0,
            y: 0,
        })
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        stdout().queue(terminal::Clear(ClearType::All))?;
        stdout().queue(cursor::MoveTo(0, 0))?;
        stdout().flush()?;
        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        self.clear()?;

        loop {
            if !self.handle_input()? {
                break;
            }
            self.draw()?;
        }

        Ok(())
    }

    fn draw(&self) -> anyhow::Result<()> {
        for (i, line) in self.buffer.iter().enumerate() {
            stdout()
                .queue(cursor::MoveTo(0, i as u16))?
                .queue(Print(line))?;
        }
        self.draw_cursor()?;
        Ok(())
    }

    fn draw_cursor(&self) -> anyhow::Result<()> {
        stdout()
            .queue(cursor::MoveTo(self.x.try_into()?, self.y.try_into()?))?
            .flush()?;
        Ok(())
    }

    fn handle_input(&mut self) -> anyhow::Result<bool> {
        let ev = read_event()?;

        match self.mode {
            Mode::Normal => self.handle_normal_input(ev),
            Mode::Insert => self.handle_insert_input(ev),
        }
    }

    fn line(&self) -> Option<&String> {
        self.buffer.get(self.y)
    }

    fn handle_normal_input(&mut self, ev: Event) -> anyhow::Result<bool> {
        match ev {
            Event::Key(KeyEvent { code: key, .. }) => match key {
                KeyCode::Char(c) => match c {
                    'q' => return Ok(false),
                    'i' => {
                        self.mode = Mode::Insert;
                    }
                    'j' => {
                        self.move_down()?;
                    }
                    'k' => {
                        self.move_up()?;
                    }
                    'h' => {
                        self.move_left()?;
                    }
                    'l' => {
                        self.move_right()?;
                    }
                    _ => {}
                },
                KeyCode::Down => {
                    self.move_down()?;
                }
                KeyCode::Up => {
                    self.move_up()?;
                }
                KeyCode::Left => {
                    self.move_left()?;
                }
                KeyCode::Right => {
                    self.move_right()?;
                }
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                }
                _ => {}
            },

            _ => {}
        }

        Ok(true)
    }

    fn handle_insert_input(&mut self, ev: Event) -> anyhow::Result<bool> {
        match ev {
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            }) => {
                self.insert_char(c)?;
                self.move_right()?;
            }
            Event::Key(KeyEvent {
                code: kc,
                modifiers: _m,
                ..
            }) => match kc {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                }
                KeyCode::Left => {
                    self.move_left()?;
                }
                KeyCode::Backspace => {
                    self.move_left()?;
                    let line = self.buffer.get_mut(self.y).expect("line out of bounds");
                    line.remove(self.x);
                }
                KeyCode::Enter => {
                    self.x = 0;
                    self.move_down()?;
                    self.buffer.insert(self.y, String::new());
                }
                _ => {}
            },
            _ => {}
        }

        Ok(true)
    }

    fn move_down(&mut self) -> anyhow::Result<()> {
        if self.buffer.len() == 0 {
            return Ok(());
        }
        if self.y < self.buffer.len() - 1 {
            self.y += 1;
            self.render_cursor()?;
        }
        Ok(())
    }

    fn move_up(&mut self) -> anyhow::Result<()> {
        if self.y > 0 {
            self.y -= 1;
            self.render_cursor()?;
        }
        Ok(())
    }

    fn move_right(&mut self) -> anyhow::Result<()> {
        let Some(line) = self.line() else {
            return Ok(());
        };
        if self.x < line.len() - 1 {
            self.x += 1;
            self.render_cursor()?;
        }
        Ok(())
    }

    fn move_left(&mut self) -> anyhow::Result<()> {
        if self.x > 0 {
            self.x -= 1;
            self.render_cursor()?;
        }
        Ok(())
    }

    fn render_cursor(&self) -> anyhow::Result<()> {
        stdout()
            .queue(cursor::MoveTo(self.x.try_into()?, self.y.try_into()?))?
            .flush()?;
        Ok(())
    }

    fn insert_char(&mut self, c: char) -> anyhow::Result<()> {
        if self.y >= self.buffer.len() {
            self.buffer.push(String::new());
        }

        let line = self.buffer.get_mut(self.y).expect("line out of bounds");
        line.insert(self.x as usize, c);
        Ok(())
    }
}

fn main() {
    let mut editor = match Editor::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to initialize editor: {}", e);
            return;
        }
    };
    editor.run().unwrap();
}

fn read_event() -> std::io::Result<Event> {
    loop {
        if let Ok(e) = event::read() {
            return Ok(e);
        }
    }
}
