use rustix::{io, ioctl};

use super::{VT, VtNumber};

macro_rules! vt_property {
    (
        $model:ty,
        $getter_name:ident = $opcode_get:expr,
        $setter_name:ident = $opcode_set:expr
    ) => {
        impl VT {
            pub fn $getter_name(&self) -> io::Result<$model> {
                unsafe {
                    ioctl::ioctl(
                        &self.descriptor,
                        ioctl::Getter::<$opcode_get, $model>::new(),
                    )
                }
            }

            pub fn $setter_name(&self, value: $model) -> io::Result<()> {
                unsafe {
                    ioctl::ioctl(
                        &self.descriptor,
                        ioctl::Setter::<$opcode_set, $model>::new(value),
                    )
                }
            }
        }
    };
}

// State

#[allow(dead_code)]
#[repr(C)]
pub struct VtState {
    pub active: u16,
    pub signal: u16,
    pub state: u16,
}

// TODO

// Render mode

#[repr(i32)]
pub enum RenderMode {
    Text = 0,
    Graphics = 1,
}

vt_property!(
    RenderMode,
    get_render_mode = 0x4B3B,
    set_render_mode = 0x4B3A
);

// Keyboard

#[repr(i32)]
#[derive(Debug)]
pub enum KeyboardMode {
    Disabled = 4,
    Scancode = 0,
    Keycode = 2,
    Ascii = 1,
    Unicode = 3,
}

vt_property!(
    KeyboardMode,
    get_keyboard_mode = 0x4B44,
    set_keybaord_mode = 0x4B45
);

// Switch

type IoChangeVt = ioctl::Setter<0x5606, VtNumber>;
type IoWaitVT = ioctl::Setter<0x5607, VtNumber>;

impl VT {
    pub fn change_to(&self, number: VtNumber) -> io::Result<()> {
        // TODO
        unsafe {
            ioctl::ioctl(&self.descriptor, IoChangeVt::new(number.clone()))?;
            ioctl::ioctl(&self.descriptor, IoWaitVT::new(number))?;
        }

        Ok(())
    }
}
