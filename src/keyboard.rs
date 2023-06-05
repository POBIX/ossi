/* code for PS/2 keyboard handling */

use crate::events::{Event, EventHandler};
use crate::interrupts::GateType;
use crate::{interrupts, io, pic};
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

    // these keys are all extended scancodes and their values are meaningless.
    MMPrevious,
    MMNext,
    NPEnter,
    Mute,
    Calculator,
    MMPlay,
    MMStop,
    VolumeDown,
    VolumeUp,
    Home,
    Up,
    PageUp,
    Left,
    Right,
    End,
    Down,
    PageDown,
    Insert,
    Delete,
    Menu,
    RCtrl,
    RAlt,
    NPSlash,
    Unknown, // (not an actual key lol)
}

impl Key {
    pub fn from_u8(val: u8) -> Option<Key> {
        // if the Key enum contains val
        if (val > 0 && val <= 0x53) || (val == 0x57 || val == 0x58) {
            unsafe {
                return Some(core::mem::transmute::<u8, Key>(val));
            }
        }
        None
    }

    pub fn to_char(&self) -> Option<char> {
        match *self {
            Key::A => Some('a'),
            Key::B => Some('b'),
            Key::C => Some('c'),
            Key::D => Some('d'),
            Key::E => Some('e'),
            Key::F => Some('f'),
            Key::G => Some('g'),
            Key::H => Some('h'),
            Key::I => Some('i'),
            Key::J => Some('j'),
            Key::K => Some('k'),
            Key::L => Some('l'),
            Key::M => Some('m'),
            Key::N => Some('n'),
            Key::O => Some('o'),
            Key::P => Some('p'),
            Key::Q => Some('q'),
            Key::R => Some('r'),
            Key::S => Some('s'),
            Key::T => Some('t'),
            Key::U => Some('u'),
            Key::V => Some('v'),
            Key::W => Some('w'),
            Key::X => Some('x'),
            Key::Y => Some('y'),
            Key::Z => Some('z'),
            Key::R1 | Key::NP1 => Some('1'),
            Key::R2 | Key::NP2 => Some('2'),
            Key::R3 | Key::NP3 => Some('3'),
            Key::R4 | Key::NP4 => Some('4'),
            Key::R5 | Key::NP5 => Some('5'),
            Key::R6 | Key::NP6 => Some('6'),
            Key::R7 | Key::NP7 => Some('7'),
            Key::R8 | Key::NP8 => Some('8'),
            Key::R9 | Key::NP9 => Some('9'),
            Key::R0 | Key::NP0 => Some('0'),
            Key::Comma => Some(','),
            Key::Dot | Key::NPDot => Some('.'),
            Key::Equals => Some('='),
            Key::NPPlus => Some('+'),
            Key::Minus | Key::NPMinus => Some('-'),
            Key::Slash => Some('/'),
            Key::Quote => Some('\''),
            Key::LeftBracket => Some('['),
            Key::RightBracket => Some(']'),
            Key::Backslash => Some('\\'),
            Key::NPAsterisk => Some('*'),
            Key::Tilde => Some('`'),
            Key::Space => Some(' '),
            Key::Enter => Some('\n'),
            Key::Tab => Some('\t'),
            Key::Semicolon => Some(';'),
            _ => None,
        }
    }

    pub fn to_shifted_char(&self) -> Option<char> {
        match *self {
            Key::A => Some('A'),
            Key::B => Some('B'),
            Key::C => Some('C'),
            Key::D => Some('D'),
            Key::E => Some('E'),
            Key::F => Some('F'),
            Key::G => Some('G'),
            Key::H => Some('H'),
            Key::I => Some('I'),
            Key::J => Some('J'),
            Key::K => Some('K'),
            Key::L => Some('L'),
            Key::M => Some('M'),
            Key::N => Some('N'),
            Key::O => Some('O'),
            Key::P => Some('P'),
            Key::Q => Some('Q'),
            Key::R => Some('R'),
            Key::S => Some('S'),
            Key::T => Some('T'),
            Key::U => Some('U'),
            Key::V => Some('V'),
            Key::W => Some('W'),
            Key::X => Some('X'),
            Key::Y => Some('Y'),
            Key::Z => Some('Z'),
            Key::R1 => Some('!'),
            Key::R2 => Some('@'),
            Key::R3 => Some('#'),
            Key::R4 => Some('$'),
            Key::R5 => Some('%'),
            Key::R6 => Some('^'),
            Key::R7 => Some('&'),
            Key::R8 => Some('*'),
            Key::R9 => Some('('),
            Key::R0 => Some(')'),
            Key::Comma => Some('<'),
            Key::Dot => Some('>'),
            Key::Equals => Some('+'),
            Key::Minus => Some('_'),
            Key::Slash => Some('?'),
            Key::Quote => Some('"'),
            Key::LeftBracket => Some('{'),
            Key::RightBracket => Some('}'),
            Key::Backslash => Some('|'),
            Key::Tilde => Some('~'),
            Key::Space => Some(' '),
            Key::Enter => Some('\n'),
            Key::Semicolon => Some(':'),
            _ => None,
        }
    }
}

const MAX_SCANCODE: usize = Key::Unknown as usize;

pub(crate) fn init() {
    unsafe {
        // attach on_key to IRQ1
        interrupts::IDT[pic::IRQ_OFFSET + 1] =
            interrupts::Handler::new(on_key, GateType::DInterrupt, 0);
    }

    ON_KEY_UP.lock().subscribe(|args| {
        if args.0 == Key::CapsLock {
            let mut caps = CAPS_LOCK.lock();
            *caps = !*caps;
        }
    });
}

fn get_state(scancode: u8) -> (u8, bool) {
    if scancode > 0x80 {
        // when a key is released, the keyboard sends the regular scancode + 0x80.
        return (scancode - 0x80, false);
    }
    (scancode, true)
}

extern "x86-interrupt" fn on_key() {
    let mut scancode = unsafe { io::inb(0x60) };
    let pressed;
    let key: Key;

    static mut PREV_EXTENDED: bool = false;

    if scancode == 0xE0 {
        // if this is an extended scancode (0xE0), we should have gotten a second byte
        // that corresponds to one of these keys
        (scancode, pressed) = unsafe { get_state(io::inb(0x60)) };
        key = match scancode {
            0x10 => Key::MMPrevious,
            0x19 => Key::MMNext,
            0x1C => Key::NPEnter,
            0x1D => Key::RCtrl,
            0x20 => Key::Mute,
            0x21 => Key::Calculator,
            0x22 => Key::MMPlay,
            0x24 => Key::MMStop,
            0x2E => Key::VolumeDown,
            0x30 => Key::VolumeUp,
            0x35 => Key::NPSlash,
            0x38 => Key::RAlt,
            0x47 => Key::Home,
            0x48 => Key::Up,
            0x49 => Key::PageUp,
            0x4B => Key::Left,
            0x4D => Key::Right,
            0x4F => Key::End,
            0x50 => Key::Down,
            0x51 => Key::PageDown,
            0x52 => Key::Insert,
            0x53 => Key::Delete,
            0x5D => Key::Menu,
            _ => Key::Unknown,
        };
        unsafe {
            PREV_EXTENDED = true;
        }
    } else {
        // if the previous call to this function received a 0xE0 (extended scan code),
        // we want to ignore this one, since it just sends a byte that we've already read.
        if unsafe { PREV_EXTENDED } {
            unsafe {
                PREV_EXTENDED = false;
            }
            pic::send_eoi(1);
            return;
        }
        (scancode, pressed) = get_state(scancode);
        key = Key::from_u8(scancode).unwrap();
    }

    set_key(key, pressed);

    if pressed {
        ON_KEY_DOWN.lock().invoke(KeyArgs(key));
    } else {
        ON_KEY_UP.lock().invoke(KeyArgs(key));
    }

    pic::send_eoi(1);
}

fn get_pos(key: Key) -> (usize, u8) {
    let k = key as usize;
    return (k / 8, k as u8 % 8);
}

pub(crate) fn is_key_pressed(key: Key) -> bool {
    let (index, bit) = get_pos(key);
    ((KEYS.lock()[index] >> bit) & 1) == 1
}

pub(crate) fn set_key(key: Key, pressed: bool) {
    let (index, bit) = get_pos(key);
    let v = pressed as u8;
    // modify the array to have that bit set to the correct value
    let mut keys = KEYS.lock();
    keys[index] = keys[index] & !(1 << bit) | (v << bit);
}

pub(crate) fn is_caps_lock_active() -> bool {
    // we can't accept keyboard input while checking the value (what if you press caps lock while reading it?)
    pic::set_mask(1, true);
    let ret = *CAPS_LOCK.lock();
    pic::set_mask(1, false);
    ret
}

// an array in which each bit corresponds to a scancode.
// e.g. while A is pressed (scancode 0x1E=30), the 6th bit of the 3rd element will be 1. (8*3+6=30)
static KEYS: Mutex<[u8; MAX_SCANCODE / 8]> = Mutex::new([0; MAX_SCANCODE / 8]);

#[derive(Clone, Copy)]
pub struct KeyArgs(pub Key);

pub(crate) static ON_KEY_DOWN: Mutex<Event<KeyArgs>> = Mutex::new(Event::<KeyArgs>::new());
pub(crate) static ON_KEY_UP: Mutex<Event<KeyArgs>> = Mutex::new(Event::<KeyArgs>::new());

static CAPS_LOCK: Mutex<bool> = Mutex::new(false);
