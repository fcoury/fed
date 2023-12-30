use std::{
    io::{stdout, Write},
    panic,
    time::Duration,
};

use command::get_command;
use crossterm::{
    cursor::{self, SetCursorStyle},
    event::{self, Event, KeyCode, KeyEvent},
    style::{Color, PrintStyledContent, Stylize},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, QueueableCommand,
};
use log::Logger;
use once_cell::sync::OnceCell;
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

#[macro_export]
macro_rules! warn {
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

    pub fn command_y(&self) -> usize {
        self.height - 1
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

    pub fn draw(&mut self, _redraw: bool) -> anyhow::Result<()> {
        log!("draw");

        // TODO: add diff detection for all changes
        self.adjust_cursor();

        self.draw_buffer()?;
        self.draw_statusline()?;
        self.draw_gutter()?;

        if self.mode.is_command() {
            self.handle_command()?;
        }

        self.draw_cursor()?;

        stdout().flush()?;
        Ok(())
    }

    pub fn draw_statusline(&mut self) -> anyhow::Result<()> {
        let y = self.height as u16 - 2;
        let line = " ".repeat(self.width);
        let mode = format!(" {:?} ", self.mode).to_uppercase();
        let pos = format!(" {}:{} ", self.by(), self.cx);
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
        let bg = hex_to_crossterm_color(&self.theme.background)?;

        let y = self.height as u16 - 1;
        let line = " ".repeat(self.width);
        stdout().queue(cursor::MoveTo(0, y))?;
        stdout().queue(PrintStyledContent(line.on(bg)))?;
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

        // if self.by() >= self.buffer.len() - 1 {
        //     self.cy = self.buffer.len() - 1;
        // }

        let max_x = self.current_line_len();

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

    fn move_down(&mut self) -> bool {
        let desired_cy = self.cy + 1;

        log!(
            "checking if inside viewport: {} < {}",
            desired_cy,
            self.vheight
        );
        // checks if we are within the viewport bounds horizontally
        if desired_cy <= self.vheight {
            log!("we are inside the viewport");

            log!(
                "checking if we are inside the buffer: {} > {}",
                self.buffer.len(),
                self.vtop + desired_cy,
            );
            // checks if we are inside the buffer
            if self.buffer.len() > self.vtop + desired_cy {
                log!("we are inside the buffer");
                if desired_cy > self.vheight - 1 {
                    self.vtop += 1;
                } else {
                    self.cy = desired_cy;
                }
                return true;
            }

            // we would go outside the buffer, does nothing
            return false;
        }

        // we are not within the bounds of the viewport, let's just scroll it one row down and keep
        // the cursor at the same position
        self.vtop += 1;
        true
    }

    fn move_up(&mut self) -> anyhow::Result<bool> {
        // if we are inside the viewport
        if self.cy > 0 {
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

    fn current_line_len(&self) -> usize {
        self.line().map(|s| s.len()).unwrap_or(0)
    }

    fn move_right(&mut self) -> anyhow::Result<bool> {
        log!("move_right");

        let mut redraw = false;

        // if we're inside the viewport
        if self.cx < self.vwidth - 1 {
            if self.bx() < self.current_line_len() {
                self.cx += 1;
            }
        } else {
            // if we're at the right edge of the viewport
            if self.vleft < self.line().map(|s| s.len() - 1).unwrap_or(0) {
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
        self.cx = self.current_line_len() - 1;
        Ok(false)
    }

    fn move_start_of_line(&mut self) -> anyhow::Result<bool> {
        self.cx = 0;
        Ok(false)
    }

    fn bx(&self) -> usize {
        self.cx + self.vleft
    }

    fn by(&self) -> usize {
        self.vtop + self.cy
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

    fn line(&self) -> Option<&String> {
        self.buffer.get(self.by())
    }

    fn handle_generic_input(&mut self, ev: &Event) -> anyhow::Result<bool> {
        match ev {
            Event::Resize(width, height) => {
                log!("resize: {}x{}", width, height);
                self.width = *width as usize;
                self.height = *height as usize;
                self.vwidth = *width as usize - self.vleft;
                self.vheight = *height as usize - 2;
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
                            self.move_to_next_page();
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
                        self.move_down();
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
                        let x = self.bx();
                        let y = self.by();
                        if let Some(line) = self.line() {
                            if x < line.len() {
                                let line = self.buffer.get_mut(y).expect("line out of bounds");
                                line.remove(x);
                            }
                            redraw = true;
                        } else {
                            warn!("line out of bounds: x: {}, y: {}", x, y);
                        }
                    }
                    'd' => match self.waiting_key {
                        Some('d') => {
                            self.buffer.remove(self.by());
                            self.waiting_key = None;
                            redraw = true;
                        }
                        _ => {
                            self.waiting_key = Some('d');
                        }
                    },
                    'J' => {
                        if let Some(line) = self.line() {
                            let empty = String::new();
                            let next_line = self.buffer.get(self.by() + 1).unwrap_or(&empty);
                            let new_line = format!("{} {}", line, next_line);
                            let y = self.by();
                            self.buffer[y] = new_line;
                            self.buffer.remove(self.by() + 1);
                            redraw = true;
                        } else {
                            warn!("line out of bounds: x: {}, y: {}", self.bx(), self.by());
                        }
                    }
                    'j' => {
                        redraw = self.move_down();
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
                    redraw = self.move_down();
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

    fn move_to_next_page(&mut self) {
        if self.buffer.len() > self.vtop + self.vheight {
            self.vtop += self.vheight;
        } else {
            self.vtop = self.buffer.len() - self.vheight;
        }
    }

    fn move_to_line(&mut self, line: usize) {
        self.vtop = line;
        self.cy = 0;
        self.move_to_start_of_line();
    }

    fn move_to_start_of_line(&mut self) {
        self.cx = 0;
    }

    fn move_to_start_of_buffer(&mut self) {
        self.vtop = 0;
        self.cy = 0;
        self.move_to_start_of_line();
    }

    fn move_to_end_of_buffer(&mut self) {
        self.vtop = self.buffer.len() - self.vheight;
        self.move_to_end_of_viewport();
    }

    fn move_to_end_of_viewport(&mut self) {
        if self.buffer.len() > self.vheight {
            self.cy = self.vheight - 1;
        } else {
            self.cy = self.buffer.len() - 1;
        }
    }

    fn move_to_previous_page(&mut self) -> anyhow::Result<()> {
        if self.vtop > self.vheight {
            self.vtop -= self.vheight;
        } else {
            self.vtop = 0;
        }
        Ok(())
    }

    fn move_to_next_word(&mut self) -> anyhow::Result<bool> {
        if let Some(line) = self.line() {
            let x = self.bx();
            let mut nx = line.chars().skip(x).position(|c| c.is_whitespace());
            if nx.is_none() {
                nx = Some(line.len() - x);
            }
            match nx {
                Some(x) => {
                    self.cx += x + 1;
                }
                None => {
                    self.cx = line.len() - 1;
                }
            }
        }
        Ok(false)
    }

    fn move_to_previous_word(&mut self) {
        if let Some(line) = self.line() {
            let x = self.bx();
            let mut px = line
                .chars()
                .rev()
                .skip(line.len() - x + 1)
                .position(|c| c.is_whitespace());
            if px.is_none() {
                px = Some(line.len() - x);
            }
            match px {
                Some(x) => {
                    self.cx -= x + 1;
                }
                None => {
                    self.cx = 0;
                }
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
        self.bx() == self.line().map(|s| s.len()).unwrap_or(0)
    }

    fn split_line_at_cursor(&mut self) -> anyhow::Result<()> {
        if self.at_end_of_line() {
            self.move_down();
            self.insert_line()?;
            return Ok(());
        }

        let x = self.bx();
        let y = self.by();

        let line = self.line().cloned();
        if let Some(line) = line {
            let (left, right) = line.split_at(x).clone();

            let line = self.buffer.get_mut(y).expect("line out of bounds");
            *line = left.to_string();

            self.buffer.insert(y + 1, right.to_string());
            self.move_down();
            self.move_start_of_line()?;
        }
        Ok(())
    }

    fn insert_char(&mut self, c: char) -> anyhow::Result<()> {
        let x = self.bx();
        let y = self.by();

        log!("Insert char {} at ({}, {})", c, x, y);
        let line = self.buffer.get_mut(y).expect("line out of bounds");
        line.insert(x as usize, c);
        Ok(())
    }

    fn insert_line(&mut self) -> anyhow::Result<()> {
        self.buffer.insert(self.by(), String::new());
        Ok(())
    }

    fn remove_char(&mut self) -> anyhow::Result<()> {
        let x = self.bx();
        let y = self.by();
        if x > 0 {
            let line = self.buffer.get_mut(y).expect("line out of bounds");
            line.remove(x - 1);
        }
        Ok(())
    }

    fn affects_buffer(&self) -> bool {
        matches!(self.mode, Mode::Normal | Mode::Insert)
    }

    fn handle_command(&mut self) -> anyhow::Result<()> {
        if let Some(cmd) = get_command(&self)? {
            log!("command: {}", cmd);
            if cmd == "q" {
                self.quit = true;
            } else if cmd == "$" {
                self.move_to_end_of_buffer();
            } else if let Ok(line) = cmd.parse::<usize>() {
                if line == 0 {
                    self.move_to_start_of_buffer();
                }
                if line < self.buffer.len() {
                    self.move_to_line(line - 1);
                }
            }
        }

        self.mode = Mode::Normal;
        self.draw(true)?;
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
