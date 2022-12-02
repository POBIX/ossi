// this file manages commands and such.
// see vga_console.rs for actual hardware interface.

use crate::{
    events::EventHandler,
    io::{Read, Seek},
    keyboard::{self, Key},
    print,
    vga_console::CONSOLE,
};

pub fn init() {
    keyboard::ON_KEY_DOWN.lock().subscribe(on_key_down);
}

fn on_key_down(args: keyboard::KeyArgs) {
    match args.0 {
        Key::Backspace => {
            let mut console = CONSOLE.lock();
            console.backspace();
            if keyboard::is_key_pressed(Key::Ctrl) {
                // if ctrl+backspace, erase until beginning of word
                while console.get_cursor_position() > 0
                    && !console
                        .read_char(console.get_cursor_position() - 1)
                        .is_whitespace()
                {
                    console.backspace();
                }
            }
            return;
        },
        Key::Left => {
            let mut console = CONSOLE.lock();
            let pos = console.get_cursor_position();
            console.seek(pos - 1);
        },
        Key::Right => {
            let mut console = CONSOLE.lock();
            let pos = console.get_cursor_position();
            console.seek(pos + 1);
        },
        _ => {
            let shift = keyboard::is_key_pressed(Key::LShift) || keyboard::is_key_pressed(Key::RShift);
            if let Some(x) = if shift { args.0.to_shifted_char() } else { args.0.to_char() } {
                if x.is_alphabetic() && keyboard::is_caps_lock_active() {
                    print!("{}", x.to_ascii_uppercase());
                }
                else {
                    print!("{x}");
                }
            }
        }
    }
}
