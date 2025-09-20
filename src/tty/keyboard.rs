use rustix::{io, ioctl};

use super::VT;

#[repr(i32)]
#[derive(Debug)]
pub enum KeyboardMode {
    Disabled = 4,
    Scancode = 0,
    Keycode = 2,
    Ascii = 1,
    Unicode = 3,
}

type IoGetKeyboardMode = ioctl::Getter<0x4B44, KeyboardMode>;
type IoSetKeyboardMode = ioctl::Setter<0x4B45, KeyboardMode>;

impl VT {
    pub fn get_keyboard_mode(&self) -> io::Result<KeyboardMode> {
        // Safety: Descriptor of VT is guaranteed to be a console
        unsafe { ioctl::ioctl(&self.descriptor, IoGetKeyboardMode::new()) }
    }

    pub fn set_keyboard_mode(&self, mode: KeyboardMode) -> io::Result<()> {
        // Safety: Descriptor of VT is guaranteed to be a console
        unsafe { ioctl::ioctl(&self.descriptor, IoSetKeyboardMode::new(mode)) }
    }
}
