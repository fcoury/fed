use std::{
    cmp,
    io::{stdout, Write},
    panic,
    time::Duration,
};

use crossterm::{
    cursor::{self, SetCursorStyle},
    event::{self, Event, KeyCode, KeyEvent},
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, QueueableCommand,
};
use log::Logger;
use once_cell::sync::OnceCell;

mod error;
mod log;
mod syntax;

static LOGGER: OnceCell<Logger> = OnceCell::new();

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        {
            let log_message = format!($($arg)*);
            $crate::LOGGER.get().expect("Logger not initialized").log(&log_message);
        }
    };
}

#[derive(Default, Debug)]
enum Mode {
    #[default]
    Normal,
    Insert,
}

#[allow(unused)]
#[derive(Default)]
struct Editor {
    mode: Mode,
    buffer: Vec<String>,
    name: String,
    width: usize,
    height: usize,
    cx: usize,
    cy: usize,
    vleft: usize,
    vtop: usize,
    vwidth: usize,
    vheight: usize,
    waiting_key: Option<char>,
}

impl Editor {
    pub fn new(file: Option<String>) -> anyhow::Result<Self> {
        let (width, height) = terminal::size()?;

        log!("terminal size = {}x{}", width, height);

        let (buffer, name) = match file {
            Some(file) => {
                let buffer = std::fs::read_to_string(&file)?;
                (buffer.lines().map(|s| s.to_string()).collect(), file)
            }
            None => (vec![String::new()], "No Name".to_string()),
        };

        Ok(Self {
            mode: Mode::Normal,
            buffer,
            name,
            width: width as usize,
            height: height as usize,
            cx: 0, // cursor x position on the viewport
            cy: 0, // cursor y position on the viewport
            vleft: 0,
            vtop: 0,
            vwidth: width as usize,
            vheight: height as usize - 2,
            ..Default::default()
        })
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        stdout().queue(terminal::Clear(ClearType::All))?;
        stdout().queue(cursor::MoveTo(0, 0))?;
        stdout().flush()?;
        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        stdout().execute(EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        self.clear()?;
        self.draw()?;

        loop {
            if event::poll(Duration::from_millis(100))? {
                let ev = event::read()?;
                log!("Event: {:?}", ev);
                if !self.handle_input(ev)? {
                    break;
                }
                self.draw()?;
            }
        }

        terminal::disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn draw_statusline(&mut self) -> anyhow::Result<()> {
        let y = self.height as u16 - 2;
        let line = " ".repeat(self.width);
        let mode = format!(" {:?} ", self.mode).to_uppercase();
        let pos = format!(" {}:{} ", self.y(), self.x());
        let filename = format!(" {} ", self.name);

        let bar_bg = Color::Rgb {
            r: 68,
            g: 70,
            b: 88,
        };
        stdout().queue(cursor::MoveTo(0, y))?;
        stdout().queue(PrintStyledContent(line.on(bar_bg)))?;

        // mode
        let mode_fg = Color::Rgb { r: 0, g: 0, b: 0 };
        let mode_bg = Color::White;
        stdout().queue(cursor::MoveTo(0, y))?;
        stdout().queue(PrintStyledContent(mode.bold().with(mode_fg).on(mode_bg)))?;

        // filename
        let name_fg = Color::White;
        stdout().queue(PrintStyledContent(filename.with(name_fg).on(bar_bg)))?;

        // position
        let pos_fg = Color::Rgb { r: 0, g: 0, b: 0 };
        let pos_bg = Color::Rgb {
            r: 178,
            g: 145,
            b: 236,
        };
        stdout().queue(cursor::MoveTo(self.width as u16 - pos.len() as u16, y))?;
        stdout().queue(PrintStyledContent(pos.bold().with(pos_fg).on(pos_bg)))?;

        stdout().flush()?;
        Ok(())
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        log!("draw");
        // gets all the lines within the viewport while clamping them to the right width
        let y0 = self.vtop;
        let y1 = cmp::min(self.vtop + self.vheight, self.buffer.len());
        let viewport_size = y1 - y0;
        let viewport = self.buffer[y0..y1].iter().map(|line| {
            if line.len() > self.vwidth {
                &line[self.vleft..self.vleft + self.vwidth]
            } else {
                line
            }
        });

        for (y, line) in viewport.enumerate() {
            stdout().queue(cursor::MoveTo(0, y as u16))?;
            stdout().queue(Print(line))?;
            if line.len() < self.vwidth {
                stdout().queue(Print(" ".repeat(self.vwidth - line.len())))?;
            }
        }

        // fill the rest of the viewport
        for y in viewport_size..self.vheight {
            stdout().queue(cursor::MoveTo(0, y as u16))?;
            stdout().queue(Print(" ".repeat(self.vwidth)))?;
        }

        self.adjust_cursor();
        self.draw_statusline()?;
        self.draw_cursor()?;

        stdout().flush()?;
        Ok(())
    }

    pub fn adjust_cursor(&mut self) {
        if self.y() >= self.buffer.len() {
            self.cy = self.buffer.len() - 1;
        }

        let max_x = self.line().len();

        log!(
            "adjust_cursor: cx: {}, cy: {}, vleft: {}, vtop: {}, max_x: {}",
            self.cx,
            self.cy,
            self.vleft,
            self.vtop,
            max_x,
        );
        if self.cx >= max_x {
            match self.mode {
                Mode::Normal => self.cx = if max_x > 0 { max_x - 1 } else { 0 },
                Mode::Insert => self.cx = max_x,
            }
        }
    }

    pub fn draw_cursor(&mut self) -> anyhow::Result<()> {
        log!("draw_cursor");
        match self.mode {
            Mode::Normal => stdout().queue(SetCursorStyle::SteadyBlock)?,
            Mode::Insert => stdout().queue(SetCursorStyle::SteadyBar)?,
        };

        stdout()
            .queue(cursor::MoveTo(self.cx.try_into()?, self.cy.try_into()?))?
            .flush()?;
        Ok(())
    }

    fn move_down(&mut self) -> anyhow::Result<()> {
        log!(
            "move_down: cx: {}, cy: {}, vleft: {}, vtop: {}",
            self.cx,
            self.cy,
            self.vleft,
            self.vtop
        );
        if self.y() < self.vheight - 1 {
            if self.y() < self.buffer.len() {
                self.cy += 1;
            }
        } else {
            self.vtop += 1;
        }
        log!(
            "move_down: cx: {}, cy: {}, vleft: {}, vtop: {}, vwidth: {}, vheight: {}",
            self.cx,
            self.cy,
            self.vleft,
            self.vtop,
            self.vwidth,
            self.vheight,
        );
        Ok(())
    }

    fn move_up(&mut self) -> anyhow::Result<()> {
        // if we are inside the viewport
        if self.cy > self.vtop {
            self.cy -= 1;
        } else {
            // if we are at the top of the viewport
            if self.vtop > 0 {
                self.vtop -= 1;
            }
        }
        Ok(())
    }

    fn move_right(&mut self) -> anyhow::Result<()> {
        log!("move_right");

        // if we're inside the viewport
        if self.cx < self.vwidth - 1 {
            if self.x() < self.line().len() {
                self.cx += 1;
            }
        } else {
            // if we're at the right edge of the viewport
            if self.vleft < self.line().len() - 1 {
                self.vleft += 1;
                self.cx += 1;
            }
        }
        log!(
            "move_right: cx: {}, cy: {}, vleft: {}, vtop: {}",
            self.cx,
            self.cy,
            self.vleft,
            self.vtop
        );
        Ok(())
    }

    fn move_left(&mut self) -> anyhow::Result<()> {
        // if we're inside the viewport
        if self.cx > self.vleft {
            self.cx -= 1;
        } else {
            // if we're at the left edge of the viewport
            if self.vleft > 0 {
                self.vleft -= 1;
            }
        }
        Ok(())
    }

    fn move_end_of_line(&mut self) -> anyhow::Result<()> {
        self.cx = self.line().len() - 1;
        Ok(())
    }

    fn move_start_of_line(&mut self) -> anyhow::Result<()> {
        self.cx = 0;
        Ok(())
    }

    fn x(&self) -> usize {
        self.cx + self.vleft
    }

    fn y(&self) -> usize {
        self.cy + self.vtop
    }

    fn handle_input(&mut self, ev: Event) -> anyhow::Result<bool> {
        log!("Event: {:?}", ev);
        if self.handle_generic_input(&ev)? {
            return Ok(true);
        }

        match self.mode {
            Mode::Normal => self.handle_normal_input(ev),
            Mode::Insert => self.handle_insert_input(ev),
        }
    }

    fn line(&self) -> &String {
        self.buffer.get(self.y()).expect("line out of bounds")
    }

    fn handle_generic_input(&mut self, ev: &Event) -> anyhow::Result<bool> {
        match ev {
            Event::Resize(width, height) => {
                log!("resize: {}x{}", width, height);
                self.width = *width as usize;
                self.height = *height as usize;
                self.vwidth = *width as usize;
                self.vheight = *height as usize - 1;
                self.draw()?;
                return Ok(true);
            }
            _ => {}
        }

        Ok(false)
    }

    fn handle_normal_input(&mut self, ev: Event) -> anyhow::Result<bool> {
        log!("handle_normal_input ev: {:?}", ev);
        match ev {
            Event::Key(KeyEvent {
                code: key,
                modifiers: mods,
                ..
            }) => match key {
                KeyCode::Char(c) => match c {
                    'f' => {
                        if mods.contains(event::KeyModifiers::CONTROL) {
                            self.move_to_next_page()?;
                        }
                    }
                    'b' => {
                        if mods.contains(event::KeyModifiers::CONTROL) {
                            self.move_to_previous_page()?;
                        } else {
                            self.move_to_previous_word();
                        }
                    }
                    'q' => return Ok(false),
                    'i' => {
                        self.mode = Mode::Insert;
                    }
                    'a' => {
                        self.move_right()?;
                        self.mode = Mode::Insert;
                    }
                    'o' => {
                        self.move_down()?;
                        self.insert_line()?;
                        self.mode = Mode::Insert;
                    }
                    'O' => {
                        self.insert_line()?;
                        self.mode = Mode::Insert;
                    }
                    'x' => {
                        let x = self.x();
                        let y = self.y();
                        let line = self.line();
                        if x < line.len() {
                            let line = self.buffer.get_mut(y).expect("line out of bounds");
                            line.remove(x);
                        }
                    }
                    'd' => match self.waiting_key {
                        Some('d') => {
                            self.buffer.remove(self.y());
                            self.waiting_key = None;
                        }
                        _ => {
                            self.waiting_key = Some('d');
                        }
                    },
                    'J' => {
                        let line = self.line();
                        let empty = String::new();
                        let next_line = self.buffer.get(self.y() + 1).unwrap_or(&empty);
                        let new_line = format!("{} {}", line, next_line);
                        let y = self.y();
                        self.buffer[y] = new_line;
                        self.buffer.remove(self.y() + 1);
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
                    '$' => {
                        self.move_end_of_line()?;
                    }
                    '0' => {
                        self.move_start_of_line()?;
                    }
                    'w' => {
                        self.move_to_next_word();
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

    fn move_to_next_page(&mut self) -> anyhow::Result<()> {
        self.vtop = cmp::min(self.vtop + self.vheight, self.buffer.len() - 1);
        Ok(())
    }

    fn move_to_previous_page(&mut self) -> anyhow::Result<()> {
        if self.vtop > 0 {
            self.vtop -= self.vheight;
        }
        Ok(())
    }

    fn move_to_next_word(&mut self) {
        let nx = self
            .line()
            .chars()
            .skip(self.x())
            .position(|c| c.is_whitespace());
        match nx {
            Some(x) => {
                self.cx += x + 1;
            }
            None => {
                self.cx = self.line().len() - 1;
            }
        }
    }

    fn move_to_previous_word(&mut self) {
        let px = self
            .line()
            .chars()
            .rev()
            .skip(self.line().len() - self.x() + 1)
            .position(|c| c.is_whitespace());
        match px {
            Some(x) => {
                self.cx -= x + 1;
            }
            None => {
                self.cx = 0;
            }
        }
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
                    self.remove_char()?;
                    self.move_left()?;
                }
                KeyCode::Enter => {
                    self.split_line_at_cursor()?;
                }
                _ => {}
            },
            _ => {}
        }

        Ok(true)
    }

    fn at_end_of_line(&self) -> bool {
        self.x() == self.line().len()
    }

    fn split_line_at_cursor(&mut self) -> anyhow::Result<()> {
        if self.at_end_of_line() {
            self.move_down()?;
            self.insert_line()?;
            return Ok(());
        }

        let x = self.x();
        let y = self.y();

        let line = self.line().clone();
        let (left, right) = line.split_at(x);

        let line = self.buffer.get_mut(y).expect("line out of bounds");
        *line = left.to_string();

        self.buffer.insert(y + 1, right.to_string());
        self.move_down()?;
        self.move_start_of_line()?;
        Ok(())
    }

    fn insert_char(&mut self, c: char) -> anyhow::Result<()> {
        let x = self.x();
        let y = self.y();

        log!("Insert char {} at ({}, {})", c, x, y);
        let line = self.buffer.get_mut(y).expect("line out of bounds");
        line.insert(x as usize, c);
        Ok(())
    }

    fn insert_line(&mut self) -> anyhow::Result<()> {
        self.buffer.insert(self.y(), String::new());
        Ok(())
    }

    fn remove_char(&mut self) -> anyhow::Result<()> {
        let x = self.x();
        let y = self.y();
        if x > 0 {
            let line = self.buffer.get_mut(y).expect("line out of bounds");
            line.remove(x - 1);
        }
        Ok(())
    }
}

fn init_logger() {
    LOGGER.set(Logger::new("/tmp/fed.log")).unwrap();
}

fn setup_panic_hook() {
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        // Clean up the terminal
        let _ = terminal::disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);

        // Call the default panic hook
        default_panic(info);
    }));
}

fn main() {
    setup_panic_hook();
    init_logger();

    let file = std::env::args().nth(1);

    let mut editor = match Editor::new(file) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to initialize editor: {}", e);
            return;
        }
    };
    editor.run().unwrap();
}
