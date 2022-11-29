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
    print!("{}", match args.0 {
        Key::MMPrevious => "previous",
        Key::MMNext => "next",
        Key::NPEnter => "enter",
        Key::Mute => "mute",
        Key::Calculator => "calc",
        Key::MMPlay => "play",
        Key::MMStop => "stop",
        Key::VolumeDown => "volume down",
        Key::VolumeUp => "volume up",
        Key::Home => "home",
        Key::Up => "up",
        Key::PageUp => "pgup",
        Key::Left => "left",
        Key::Right => "right",
        Key::End => "end",
        Key::Down => "down",
        Key::PageDown => "pgdn",
        Key::Insert => "insert",
        Key::Delete  => "del",
        Key::Menu => "menu",
        Key::RCtrl => "rctrl",
        Key::RAlt => "ralt",
        Key::NPSlash => "slash",
        _ => ""
    });
    if args.0 == Key::Backspace {
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
    }
    let shift = keyboard::is_key_pressed(Key::LShift) || keyboard::is_key_pressed(Key::RShift);
    if let Some(x) = if shift {
        args.0.to_shifted_char()
    } else {
        args.0.to_char()
    } {
        print!("{x}");
    }
}
