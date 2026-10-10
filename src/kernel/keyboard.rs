use crate::events::{Event, EventHandler};
use crate::interrupts::GateType;
use crate::kernel::{interrupts, io, pic};
use spin::Mutex;
use crate::keyboard::*;

const MAX_SCANCODE: usize = Key::Unknown as usize;

pub(crate) fn init() {
    unsafe {
        // attach on_key to IRQ1
        interrupts::IDT[crate::pic::IRQ_OFFSET + 1] =
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
    // we can't accept keyboard input while checking (what if you press caps lock while reading it?)
    pic::set_mask(1, true);
    let ret = *CAPS_LOCK.lock();
    pic::set_mask(1, false);
    ret
}

// an array in which each bit corresponds to a scancode.
// e.g. while A is pressed (scancode 0x1E=30), the 6th bit of the 3rd element will be 1. (8*3+6=30)
static KEYS: Mutex<[u8; MAX_SCANCODE / 8]> = Mutex::new([0; MAX_SCANCODE / 8]);

pub(crate) static ON_KEY_DOWN: Mutex<Event<KeyArgs>> = Mutex::new(Event::<KeyArgs>::new());
pub(crate) static ON_KEY_UP: Mutex<Event<KeyArgs>> = Mutex::new(Event::<KeyArgs>::new());

static CAPS_LOCK: Mutex<bool> = Mutex::new(false);
