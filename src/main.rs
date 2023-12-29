use std::{
    cmp,
    io::{stdout, Write},
    panic,
    time::Duration,
};

use command::new_command_editor;
use crossterm::{
    cursor::{self, SetCursorStyle},
    event::{self, Event, KeyCode, KeyEvent},
    style::{Color, PrintStyledContent, Stylize},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, QueueableCommand,
};
use log::Logger;
use once_cell::sync::OnceCell;
use rustyline::error::ReadlineError;
use theme::Theme;
use utils::{darken, hex_to_crossterm_color};

use crate::syntax::{highlight, Viewport};

mod command;
mod error;
mod log;
mod syntax;
mod theme;
mod utils;

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
    Command,
}

impl Mode {
    fn is_command(&self) -> bool {
        matches!(self, Mode::Command)
    }
}

#[derive(Default)]
struct Config {
    use_faded_line_numbers: bool,
}

#[allow(unused)]
#[derive(Default)]
struct Editor {
    theme: Theme,
    config: Config,
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
    pending_redraw: bool,
    quit: bool,
}

impl Editor {
    pub fn new(theme: Theme, file: Option<String>) -> anyhow::Result<Self> {
        let (width, height) = terminal::size()?;

        log!("terminal size = {}x{}", width, height);

        let (buffer, name) = match file {
            Some(file) => {
                let buffer = std::fs::read_to_string(&file)?;
                (buffer.lines().map(|s| s.to_string()).collect(), file)
            }
            None => (vec![String::new()], "No Name".to_string()),
        };

        let vleft = 8;

        // TODO: read from disk
        let config = Config {
            use_faded_line_numbers: true,
        };

        Ok(Self {
            mode: Mode::Normal,
            theme,
            buffer,
            name,
            width: width as usize,
            height: height as usize,
            cx: 0, // cursor x position on the viewport
            cy: 0, // cursor y position on the viewport
            vleft,
            vtop: 0,
            vwidth: width as usize - vleft,
            vheight: height as usize - 2,
            config,
            ..Default::default()
        })
    }

    pub fn line_number(&self) -> usize {
        self.vtop + self.cy + 1
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        stdout().queue(terminal::Clear(ClearType::All))?;
        stdout().queue(cursor::MoveTo(0, 0))?;
        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        stdout().execute(EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        self.clear()?;
        self.draw(true)?;

        loop {
            if event::poll(Duration::from_millis(100))? {
                let ev = event::read()?;
                log!("Event: {:?}", ev);
                match self.handle_input(ev) {
                    Ok(redraw) => {
                        self.draw(redraw)?;
                    }
                    Err(err) => {
                        log!("Error: {}", err);
                        break;
                    }
                }
            }

            if self.quit {
                break;
            }
        }

        terminal::disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn draw_editor(&mut self) -> anyhow::Result<()> {
        self.draw_gutter()?;
        self.draw_buffer()?;
        self.draw_statusline()?;
        self.draw_commandline()?;
        Ok(())
    }

    pub fn draw(&mut self, redraw: bool) -> anyhow::Result<()> {
        log!("draw");

        if redraw || self.pending_redraw {
            self.pending_redraw = false;
            self.draw_editor()?;
        }

        if self.mode.is_command() {
            self.handle_commandline();
            self.draw_editor()?;
        }

        self.adjust_cursor();
        self.draw_cursor();

        stdout().flush()?;
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
        let mode_bg = Color::Rgb {
            r: 178,
            g: 145,
            b: 236,
        };
        stdout().queue(cursor::MoveTo(0, y))?;
        stdout().queue(PrintStyledContent(mode.bold().with(mode_fg).on(mode_bg)))?;
        stdout().queue(PrintStyledContent("".with(mode_bg).on(bar_bg)))?;

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
        stdout().queue(cursor::MoveTo(self.width as u16 - pos.len() as u16 - 1, y))?;
        stdout().queue(PrintStyledContent("".with(pos_bg).on(bar_bg)))?;
        stdout().queue(PrintStyledContent(pos.bold().with(pos_fg).on(pos_bg)))?;

        Ok(())
    }

    pub fn draw_commandline(&mut self) -> anyhow::Result<()> {
        // let fg = hex_to_crossterm_color(&self.theme.foreground)?;
        let bg = hex_to_crossterm_color(&self.theme.background)?;

        let y = self.height as u16 - 1;
        let line = " ".repeat(self.width);
        stdout().queue(cursor::MoveTo(0, y))?;
        stdout().queue(PrintStyledContent(line.on(bg)))?;
        // stdout().queue(cursor::MoveTo(0, y))?;
        // stdout().queue(PrintStyledContent(":".with(fg).on(bg)))?;
        Ok(())
    }

    pub fn draw_gutter(&mut self) -> anyhow::Result<()> {
        let fg = hex_to_crossterm_color(
            &self
                .theme
                .gutter_foreground
                .clone()
                .unwrap_or(self.theme.foreground.clone()),
        )?;
        let fgh = hex_to_crossterm_color(
            &self
                .theme
                .gutter_foreground_highlight
                .clone()
                .unwrap_or(self.theme.foreground.clone()),
        )?;
        let bg = hex_to_crossterm_color(
            &self
                .theme
                .gutter_background
                .clone()
                .unwrap_or(self.theme.background.clone()),
        )?;

        let width = self.vleft - 2;
        for y in 0..self.vheight {
            let fg = if self.config.use_faded_line_numbers {
                darken(fg, 0.5)?
            } else {
                fg
            };
            let color = if y == self.cy { fgh } else { fg };
            let line_number = format!("{:>width$}", y + self.vtop + 1);
            stdout().queue(cursor::MoveTo(0, y as u16))?;
            stdout().queue(PrintStyledContent(line_number.with(color).on(bg)))?;
            stdout().queue(PrintStyledContent(" ▎".to_string().with(fg).on(bg)))?;
        }

        Ok(())
    }

    pub fn draw_buffer(&mut self) -> anyhow::Result<()> {
        log!(
            "draw_buffer left={} width={} total={}",
            self.vleft,
            self.vwidth,
            self.width
        );

        let viewport = Viewport::new(self.vtop, self.vleft, self.vwidth, self.vheight);
        highlight(&self.buffer, &self.theme, &viewport)?;

        Ok(())
    }

    pub fn adjust_cursor(&mut self) {
        if !self.affects_buffer() {
            return;
        }

        if self.y() >= self.buffer.len() {
            self.cy = self.buffer.len() - 1;
        }

        if self.cx < 0 {
            self.cx = 0;
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
                Mode::Command => {}
            }
        }
    }

    pub fn draw_cursor(&mut self) -> anyhow::Result<()> {
        if !self.affects_buffer() {
            return Ok(());
        }

        log!("draw_cursor cx={} cy={}", self.cx, self.cy);
        match self.mode {
            Mode::Normal => {
                stdout().queue(SetCursorStyle::SteadyBlock)?;
            }
            Mode::Insert => {
                stdout().queue(SetCursorStyle::SteadyBar)?;
            }
            Mode::Command => {}
        }

        stdout().queue(cursor::MoveTo(
            (self.vleft + self.cx).try_into()?,
            self.cy.try_into()?,
        ))?;
        Ok(())
    }

    fn move_down(&mut self) -> anyhow::Result<bool> {
        log!(
            "move_down: cx: {}, cy: {}, vleft: {}, vtop: {}",
            self.cx,
            self.cy,
            self.vleft,
            self.vtop
        );
        let mut redraw = false;
        if self.y() < self.vheight - 1 {
            if self.y() < self.buffer.len() {
                self.cy += 1;
            }
        } else {
            self.vtop += 1;
            redraw = true;
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
        Ok(redraw)
    }

    fn move_up(&mut self) -> anyhow::Result<bool> {
        // if we are inside the viewport
        if self.cy > self.vtop {
            self.cy -= 1;
        } else {
            // if we are at the top of the viewport
            if self.vtop > 0 {
                self.vtop -= 1;
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn move_right(&mut self) -> anyhow::Result<bool> {
        log!("move_right");

        let mut redraw = false;

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
                redraw = true;
            }
        }
        log!(
            "move_right: cx: {}, cy: {}, vleft: {}, vtop: {}",
            self.cx,
            self.cy,
            self.vleft,
            self.vtop
        );
        Ok(redraw)
    }

    fn move_left(&mut self) -> anyhow::Result<bool> {
        // if we're inside the viewport
        if self.cx > 0 {
            self.cx -= 1;
        }
        Ok(false)
    }

    fn move_end_of_line(&mut self) -> anyhow::Result<bool> {
        self.cx = self.line().len() - 1;
        Ok(false)
    }

    fn move_start_of_line(&mut self) -> anyhow::Result<bool> {
        self.cx = 0;
        Ok(false)
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
            Mode::Command => Ok(true),
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
                self.vwidth = *width as usize - self.vleft;
                self.vheight = *height as usize - 1;
                self.draw(true)?;
                return Ok(true);
            }
            _ => {}
        }

        Ok(false)
    }

    /// Handles normal mode input. Returns true if a redraw is needed.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error on the underlying
    /// command execution.
    fn handle_normal_input(&mut self, ev: Event) -> anyhow::Result<bool> {
        log!("handle_normal_input ev: {:?}", ev);
        let mut redraw = false;
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
                            redraw = true;
                        }
                    }
                    'b' => {
                        if mods.contains(event::KeyModifiers::CONTROL) {
                            self.move_to_previous_page()?;
                        } else {
                            self.move_to_previous_word();
                        }
                        redraw = true;
                    }
                    'q' => return Ok(false),
                    'i' => {
                        self.mode = Mode::Insert;
                    }
                    'a' => {
                        self.move_right()?;
                        self.mode = Mode::Insert;
                    }
                    ':' | ';' => {
                        self.mode = Mode::Command;
                    }
                    'o' => {
                        self.move_down()?;
                        self.insert_line()?;
                        self.mode = Mode::Insert;
                        redraw = true;
                    }
                    'O' => {
                        self.insert_line()?;
                        self.mode = Mode::Insert;
                        redraw = true;
                    }
                    'x' => {
                        let x = self.x();
                        let y = self.y();
                        let line = self.line();
                        if x < line.len() {
                            let line = self.buffer.get_mut(y).expect("line out of bounds");
                            line.remove(x);
                        }
                        redraw = true;
                    }
                    'd' => match self.waiting_key {
                        Some('d') => {
                            self.buffer.remove(self.y());
                            self.waiting_key = None;
                            redraw = true;
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
                        redraw = true;
                    }
                    'j' => {
                        redraw = self.move_down()?;
                    }
                    'k' => {
                        redraw = self.move_up()?;
                    }
                    'h' => {
                        redraw = self.move_left()?;
                    }
                    'l' => {
                        redraw = self.move_right()?;
                    }
                    '$' => {
                        redraw = self.move_end_of_line()?;
                    }
                    '0' => {
                        redraw = self.move_start_of_line()?;
                    }
                    'w' => {
                        redraw = self.move_to_next_word()?;
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

        Ok(redraw)
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

    fn move_to_next_word(&mut self) -> anyhow::Result<bool> {
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
        Ok(false)
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

    fn affects_buffer(&self) -> bool {
        matches!(self.mode, Mode::Normal | Mode::Insert)
    }

    fn handle_commandline(&mut self) -> anyhow::Result<()> {
        let mut rl = new_command_editor()?;
        stdout().execute(cursor::MoveTo(0, self.height as u16 - 1))?;
        match rl.readline(":") {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                log!("Command: {}", line);
                self.handle_command(&line)?;
            }
            Err(ReadlineError::Interrupted) => {
                log!("CTRL-C");
            }
            Err(ReadlineError::Eof) => {
                log!("CTRL-D");
            }
            Err(err) => {
                log!("Error: {:?}", err);
            }
        }

        self.mode = Mode::Normal;
        self.pending_redraw = true;
        Ok(())
    }

    fn handle_command(&mut self, cmd: &str) -> anyhow::Result<()> {
        if cmd == "q" {
            self.quit = true;
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
    let theme = std::env::args()
        .nth(2)
        .unwrap_or("src/fixtures/GitHub.tmTheme".to_string());
    log!("theme: {}", theme);
    let theme = Theme::parse(theme).unwrap();
    log!("theme: {:#?}", theme);

    let mut editor = match Editor::new(theme, file) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to initialize editor: {}", e);
            return;
        }
    };
    editor.run().unwrap();
}
