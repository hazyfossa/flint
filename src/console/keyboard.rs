use super::VT;
use crate::utils::ioctl;

#[repr(i32)]
#[derive(Debug)]
enum KeyboardMode {
    Disabled = 4,
    Scancode = 0,
    Keycode = 2,
    Ascii = 1,
    Unicode = 3,
}

impl TryFrom<i32> for KeyboardMode {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        unsafe {
            match value {
                0..=4 => Ok(std::mem::transmute::<i32, KeyboardMode>(value)),
                _ => Err(()),
            }
        }
    }
}

crate::define_ioctl!(struct IoGetKeyboardMode {
    opcode: 0x4B44,
    mutating: true,
    output: KeyboardMode,
});

crate::define_ioctl!(struct IoSetKeyboardMode {
    opcode: 0x4B45,
    mutating: false,
    input: KeyboardMode,
});

impl VT {
    fn get_keyboard_mode(&self) -> ioctl::Result<KeyboardMode> {
        // Safety: Descriptor of VT is guaranteed to be a console
        unsafe { ioctl::run(&self.descriptor, IoGetKeyboardMode::new()) }
    }

    fn set_keyboard_mode(&self, mode: KeyboardMode) -> ioctl::Result<()> {
        // Safety: Descriptor of VT is guaranteed to be a console
        unsafe { ioctl::run(&self.descriptor, IoSetKeyboardMode::new(mode)) }
    }
}
