/* code for PS/2 keyboard handling */

use crate::interrupts::GateType;
use crate::{interrupts, io, pic, print};
use spin::Mutex;

/// Key down map for scancode set 1
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Key {
    Escape = 0x01,
    R1 = 0x02,
    R2 = 0x03,
    R3 = 0x04,
    R4 = 0x05,
    R5 = 0x06,
    R6 = 0x07,
    R7 = 0x08,
    R8 = 0x09,
    R9 = 0x0A,
    R0 = 0x0B,
    Minus = 0x0C,
    Equals = 0x0D,
    Backspace = 0x0E,
    Tab = 0x0F,
    Q = 0x10,
    W = 0x11,
    E = 0x12,
    R = 0x13,
    T = 0x14,
    Y = 0x15,
    U = 0x16,
    I = 0x17,
    O = 0x18,
    P = 0x19,
    LeftBracket = 0x1A,
    RightBracket = 0x1B,
    Enter = 0x1C,
    Ctrl = 0x1D,
    A = 0x1E,
    S = 0x1F,
    D = 0x20,
    F = 0x21,
    G = 0x22,
    H = 0x23,
    J = 0x24,
    K = 0x25,
    L = 0x26,
    Semicolon = 0x27,
    Quote = 0x28,
    Tilde = 0x29,
    LShift = 0x2A,
    Backslash = 0x2B,
    Z = 0x2C,
    X = 0x2D,
    C = 0x2E,
    V = 0x2F,
    B = 0x30,
    N = 0x31,
    M = 0x32,
    Comma = 0x33,
    Dot = 0x34,
    Slash = 0x35,
    RShift = 0x36,
    NPAsterisk = 0x37,
    Alt = 0x38,
    Space = 0x39,
    CapsLock = 0x3A,
    F1 = 0x3B,
    F2 = 0x3C,
    F3 = 0x3D,
    F4 = 0x3E,
    F5 = 0x3F,
    F6 = 0x40,
    F7 = 0x41,
    F8 = 0x42,
    F9 = 0x43,
    F10 = 0x44,
    NumLock = 0x45,
    ScrollLock = 0x46,
    NP7 = 0x47,
    NP8 = 0x48,
    NP9 = 0x49,
    NPMinus = 0x4A,
    NP4 = 0x4B,
    NP5 = 0x4C,
    NP6 = 0x4D,
    NPPlus = 0x4E,
    NP1 = 0x4F,
    NP2 = 0x50,
    NP3 = 0x51,
    NP0 = 0x52,
    NPDot = 0x53,
    F11 = 0x57,
    F12 = 0x58,
}

impl Key {
    pub fn from_u8(val: u8) -> Option<Key> {
        if (val > 0 && val < 0x54) || (val == 0x57 || val == 0x58) {
            unsafe {
                return Some(core::mem::transmute::<u8, Key>(val));
            }
        }
        None
    }

    pub fn to_char(&self) -> char {
        match *self {
            Key::A => 'a',
            Key::B => 'b',
            Key::C => 'c',
            Key::D => 'd',
            Key::E => 'e',
            Key::F => 'f',
            Key::G => 'g',
            Key::H => 'h',
            Key::I => 'i',
            Key::J => 'j',
            Key::K => 'k',
            Key::L => 'l',
            Key::M => 'm',
            Key::N => 'n',
            Key::O => 'o',
            Key::P => 'p',
            Key::Q => 'q',
            Key::R => 'r',
            Key::S => 's',
            Key::T => 't',
            Key::U => 'u',
            Key::V => 'v',
            Key::W => 'w',
            Key::X => 'x',
            Key::Y => 'y',
            Key::Z => 'z',
            Key::R1 | Key::NP1 => '1',
            Key::R2 | Key::NP2 => '2',
            Key::R3 | Key::NP3 => '3',
            Key::R4 | Key::NP4 => '4',
            Key::R5 | Key::NP5 => '5',
            Key::R6 | Key::NP6 => '6',
            Key::R7 | Key::NP7 => '7',
            Key::R8 | Key::NP8 => '8',
            Key::R9 | Key::NP9 => '9',
            Key::R0 | Key::NP0 => '0',
            Key::Comma => ',',
            Key::Dot | Key::NPDot => '.',
            Key::Equals => '=',
            Key::NPPlus => '+',
            Key::Minus | Key::NPMinus => '-',
            Key::Slash => '/',
            Key::Quote => '\'',
            Key::LeftBracket => '[',
            Key::RightBracket => ']',
            Key::Backslash => '\\',
            Key::NPAsterisk => '*',
            Key::Tilde => '`',
            Key::Space => ' ',
            Key::Enter => '\n',
            Key::Tab => '\t',
            Key::Semicolon => ';',
            _ => '\0'
        }
    }
}

const MAX_SCANCODE: usize = Key::F12 as usize;

pub fn init() {
    unsafe {
        // attach on_key to IRQ1
        interrupts::IDT[pic::IRQ_OFFSET + 1] =
            interrupts::Handler::new(on_key, GateType::DInterrupt);
    }
}

extern "x86-interrupt" fn on_key() {
    let mut scancode = unsafe { io::inb(0x60) };
    let mut pressed = true;

    if scancode == 0xE0 {
        // if this is an extended scancode, ignore it for now.

        unsafe { io::inb(0x60); } // clear the keyboard's buffer.

        pic::send_eoi(1);
        return;
    }

    if scancode > MAX_SCANCODE as u8 {
        // when a key is released, the keyboard sends the regular scancode + 0x80.
        scancode = scancode - 0x80;
        pressed = false;
    }
    else {
        let key = Key::from_u8(scancode).unwrap();
        if key == Key::Backspace {
            crate::vga_console::CONSOLE.lock().backspace();
        }
        else {
            print!("{}", key.to_char());
        }
    }

    let key = Key::from_u8(scancode).unwrap();
    set_key(key, pressed);

    pic::send_eoi(1);
}

fn get_pos(key: Key) -> (usize, u8) {
    let k = key as usize;
    return (k / 8, k as u8 % 8);
}

pub fn is_key_pressed(key: Key) -> bool {
    unsafe {
        let (index, bit) = get_pos(key);
        ((KEYS.lock()[index] >> bit) & 1) == 1
    } // check whether that bit is set
}

pub fn set_key(key: Key, pressed: bool) {
    let (index, bit) = get_pos(key);
    let v = pressed as u8;
    // modify the array to have that bit set to the correct value
    unsafe {
        let mut keys = KEYS.lock();
        keys[index] = keys[index] & !(1 << bit) | (v << bit);
    }
}

// an array in which each bit corresponds to a scancode.
// e.g. while A is pressed (scancode 0x1E=30), the 6th bit of the 3rd element will be 1. (8*3+6=30)
static mut KEYS: Mutex<[u8; MAX_SCANCODE / 8]> = Mutex::new([0; MAX_SCANCODE / 8]);
