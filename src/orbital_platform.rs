//! Custom Slint platform backend for Redox/Orbital.
//!
//! Modern Slint's winit backend does not support Redox (winit 0.30's orbital
//! platform lacks the pump_events API slint requires), and the old proven pair
//! (slint 1.1 + winit 0.28) opens the legacy `event:` kernel scheme that the
//! current kernel removed. So E-OS apps drive Slint's software renderer
//! directly over orbclient — the same client library the rest of the Crimson
//! desktop (launcher, orblogin, eos-settings) is built on.

use orbclient::{EventOption, Renderer, Window, WindowFlag};
use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Key, Platform, PointerEventButton, WindowEvent};
use slint::{LogicalPosition, PhysicalSize};
use std::rc::Rc;
use std::time::{Duration, Instant};

pub struct OrbitalPlatform {
    window: Rc<MinimalSoftwareWindow>,
    start: Instant,
}

impl OrbitalPlatform {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            window: MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer),
            start: Instant::now(),
        }
    }
}

impl Platform for OrbitalPlatform {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> Duration {
        self.start.elapsed()
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        run_loop(self.window.clone())
    }
}

/// Map an orbclient key event to a Slint key text, when representable.
fn key_text(key: &orbclient::KeyEvent) -> Option<slint::SharedString> {
    use orbclient::{
        K_BKSP, K_DEL, K_DOWN, K_END, K_ENTER, K_ESC, K_HOME, K_LEFT, K_PGDN, K_PGUP, K_RIGHT,
        K_TAB, K_UP,
    };
    match key.scancode {
        K_BKSP => Some(Key::Backspace.into()),
        K_ENTER => Some(Key::Return.into()),
        K_TAB => Some(Key::Tab.into()),
        K_ESC => Some(Key::Escape.into()),
        K_LEFT => Some(Key::LeftArrow.into()),
        K_RIGHT => Some(Key::RightArrow.into()),
        K_UP => Some(Key::UpArrow.into()),
        K_DOWN => Some(Key::DownArrow.into()),
        K_DEL => Some(Key::Delete.into()),
        K_HOME => Some(Key::Home.into()),
        K_END => Some(Key::End.into()),
        K_PGUP => Some(Key::PageUp.into()),
        K_PGDN => Some(Key::PageDown.into()),
        _ => {
            if key.character != '\0' {
                Some(slint::SharedString::from(key.character))
            } else {
                None
            }
        }
    }
}

fn run_loop(sw: Rc<MinimalSoftwareWindow>) -> Result<(), slint::PlatformError> {
    // Orbital's session children inherit DISPLAY; a shell launch (VT console,
    // ssh) does not have it. orbclient's window path already defaults to
    // /scheme/orbital — give get_display_size() the same default.
    if std::env::var_os("DISPLAY").is_none() {
        let display =
            std::env::var("ORBITAL_DISPLAY").unwrap_or_else(|_| "/scheme/orbital".to_string());
        std::env::set_var("DISPLAY", display);
    }
    let (dw, dh) = orbclient::get_display_size()
        .map_err(|e| slint::PlatformError::Other(format!("orbital display size: {e}")))?;
    let w = dw.saturating_sub(80).clamp(480, 900);
    let h = dh.saturating_sub(140).clamp(320, 620);
    let x = ((dw.saturating_sub(w)) / 2) as i32;
    let y = ((dh.saturating_sub(h)) / 2) as i32;

    let mut win = Window::new_flags(
        x,
        y,
        w,
        h,
        "E-OS Notes",
        &[WindowFlag::Async, WindowFlag::Resizable],
    )
    .ok_or_else(|| slint::PlatformError::Other("cannot open an orbital window".into()))?;

    sw.set_size(PhysicalSize::new(w, h));
    let mut buffer = vec![PremultipliedRgbaColor::default(); (w * h) as usize];
    let mut stride = w as usize;

    let mut pointer = LogicalPosition::new(0.0, 0.0);
    let (mut left, mut middle, mut right) = (false, false, false);

    loop {
        slint::platform::update_timers_and_animations();

        for event in win.events() {
            match event.to_option() {
                // Printable characters arrive as a separate TextInput event:
                // orbital runs the scancode through inputd's keymap, then sends
                // the glyph here and a *cleared* (character='\0') KeyEvent below.
                // The KeyEvent path only carries navigation/editing keys.
                EventOption::TextInput(ti) => {
                    if ti.character != '\0' {
                        let text = slint::SharedString::from(ti.character);
                        sw.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
                        sw.dispatch_event(WindowEvent::KeyReleased { text });
                    }
                }
                EventOption::Key(key) => {
                    if let Some(text) = key_text(&key) {
                        if key.pressed {
                            sw.dispatch_event(WindowEvent::KeyPressed { text });
                        } else {
                            sw.dispatch_event(WindowEvent::KeyReleased { text });
                        }
                    }
                }
                EventOption::Mouse(mouse) => {
                    pointer = LogicalPosition::new(mouse.x as f32, mouse.y as f32);
                    sw.dispatch_event(WindowEvent::PointerMoved { position: pointer });
                }
                EventOption::Button(btn) => {
                    for (was, is, button) in [
                        (left, btn.left, PointerEventButton::Left),
                        (middle, btn.middle, PointerEventButton::Middle),
                        (right, btn.right, PointerEventButton::Right),
                    ] {
                        if is && !was {
                            sw.dispatch_event(WindowEvent::PointerPressed {
                                position: pointer,
                                button,
                            });
                        } else if !is && was {
                            sw.dispatch_event(WindowEvent::PointerReleased {
                                position: pointer,
                                button,
                            });
                        }
                    }
                    left = btn.left;
                    middle = btn.middle;
                    right = btn.right;
                }
                EventOption::Scroll(scroll) => {
                    sw.dispatch_event(WindowEvent::PointerScrolled {
                        position: pointer,
                        delta_x: (scroll.x * 24) as f32,
                        delta_y: (scroll.y * 24) as f32,
                    });
                }
                EventOption::Resize(resize) => {
                    stride = resize.width as usize;
                    buffer = vec![
                        PremultipliedRgbaColor::default();
                        (resize.width * resize.height) as usize
                    ];
                    sw.set_size(PhysicalSize::new(resize.width, resize.height));
                }
                EventOption::Quit(_) => return Ok(()),
                _ => {}
            }
        }

        sw.draw_if_needed(|renderer| {
            renderer.render(&mut buffer, stride);
            let data = win.data_mut();
            let n = data.len().min(buffer.len());
            for i in 0..n {
                let px = &buffer[i];
                data[i] = orbclient::Color {
                    data: (px.alpha as u32) << 24
                        | (px.red as u32) << 16
                        | (px.green as u32) << 8
                        | (px.blue as u32),
                };
            }
            win.sync();
        });

        std::thread::sleep(Duration::from_millis(if sw.has_active_animations() {
            16
        } else {
            32
        }));
    }
}
