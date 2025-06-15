use crate::RenderFrame;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use iv2c::{error::Error, pipeline::Resolution, render::CallbackState};
use std::{
    io::{Result as IOResult, Write, stdout},
    time::Duration,
};

#[derive(Debug)]
pub struct TerminalPlayer {
    fg_color: Color,
    bg_color: Color,
    title: String,
    use_grayscale: bool,
}

#[derive(PartialEq, Eq, Debug)]
enum Control {
    None,
    Exit,
    Resize(u16, u16),
}

impl TerminalPlayer {
    pub fn new(title: String, use_grayscale: bool) -> Self {
        Self {
            fg_color: Color::White,
            bg_color: Color::Black,
            title,
            use_grayscale,
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        execute!(stdout(), EnterAlternateScreen, SetTitle(&self.title))?;
        terminal::enable_raw_mode()?;
        self.clear()?;
        Ok(())
    }

    pub fn size() -> Result<(u16, u16), Error> {
        terminal::size().map_err(Into::into)
    }

    pub fn callback(&self) -> impl Fn(CallbackState) -> bool {
        |CallbackState {
             frame,
             should_render,
             pipeline,
         }| {
            match self.poll_events() {
                Control::Exit => return false,
                Control::Resize(height, width) => {
                    pipeline.set_resolution(Resolution::Fixed(height as u32, width as u32));
                }
                Control::None => {}
            }

            if should_render && let Some(f) = frame {
                let _ = self.draw(&f);
            }

            true
        }
    }

    fn clear(&self) -> IOResult<()> {
        execute!(
            stdout(),
            Clear(ClearType::All),
            Hide,
            SetForegroundColor(self.fg_color),
            SetBackgroundColor(self.bg_color),
            MoveTo(0, 0),
        )?;
        stdout().flush()?;
        Ok(())
    }

    fn cleanup(&self) -> IOResult<()> {
        // Restore terminal state
        execute!(
            stdout(),
            ResetColor,
            Clear(ClearType::All),
            Show,
            LeaveAlternateScreen
        )?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn poll_events(&self) -> Control {
        if event::poll(Duration::from_millis(10)).is_ok_and(|r| r) {
            let Ok(ev) = event::read() else {
                return Control::None;
            };

            return match ev {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q') | KeyCode::Char('Q'),
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('c') | KeyCode::Char('C'),
                    modifiers: event::KeyModifiers::CONTROL,
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => Control::Exit,
                Event::Resize(width, height) => Control::Resize(width, height),
                _ => Control::None,
            };
        }

        Control::None
    }

    fn draw(&self, RenderFrame { text, colors }: &RenderFrame) -> IOResult<()> {
        let print_string = |string: &str| {
            let mut out = stdout();
            execute!(out, MoveTo(0, 0), Print(string), MoveTo(0, 0))?;
            out.flush()?;
            Ok(())
        };

        if self.use_grayscale {
            print_string(text)
        } else {
            let mut colored_string = String::with_capacity(text.len() * 10);
            for (c, rgb) in text.chars().zip(colors.chunks(3)) {
                let color = Color::Rgb {
                    r: rgb[0],
                    g: rgb[1],
                    b: rgb[2],
                };
                colored_string.push_str(&format!("{}", c.stylize().with(color)));
            }
            print_string(&colored_string)
        }
    }
}

impl Drop for TerminalPlayer {
    fn drop(&mut self) {
        self.cleanup().expect("Failed to clean up Terminal.");
    }
}
