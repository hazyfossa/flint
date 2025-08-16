use std::{mem, ptr};

use rustix::{
    ffi, io,
    ioctl::{self, Ioctl},
};

use super::VT;

#[repr(i32)]
#[derive(Debug)]
enum KeyboardMode {
    Disabled = 4,
    Scancode = 0,
    Keycode = 2,
    Ascii = 1,
    Unicode = 3,
}

static IO_GET_KEYBOARD_MODE: u32 = 0x4B44;
static IO_SET_KEYBOARD_MODE: u32 = 0x4B45;

struct IoGetKeyboardMode;

unsafe impl Ioctl for IoGetKeyboardMode {
    const IS_MUTATING: bool = true;
    type Output = KeyboardMode;

    fn opcode(&self) -> ioctl::Opcode {
        0x4B44
    }

    fn as_ptr(&mut self) -> *mut ffi::c_void {
        ptr::null_mut() as _
    }

    unsafe fn output_from_ptr(
        out: ioctl::IoctlOutput,
        _extract_output: *mut ffi::c_void,
    ) -> io::Result<Self::Output> {
        unsafe {
            match out {
                0..=4 => Ok(mem::transmute::<i32, KeyboardMode>(out)),
                _ => Err(io::Errno::INVAL),
            }
        }
    }
}

struct IoSetKeyboardMode(ffi::c_int);

impl IoSetKeyboardMode {
    fn to(mode: KeyboardMode) -> Self {
        Self(mode as _)
    }
}

unsafe impl Ioctl for IoSetKeyboardMode {
    const IS_MUTATING: bool = false;
    type Output = ();

    fn opcode(&self) -> rustix::ioctl::Opcode {
        0x4B45
    }

    fn as_ptr(&mut self) -> *mut rustix::ffi::c_void {
        &mut self.0 as *mut ffi::c_int as _
    }

    unsafe fn output_from_ptr(
        out: rustix::ioctl::IoctlOutput,
        _: *mut rustix::ffi::c_void,
    ) -> io::Result<Self::Output> {
        if out != 0 {
            Err(io::Errno::from_raw_os_error(out))
        } else {
            Ok(())
        }
    }
}

impl VT {
    fn get_keyboard_mode(&self) -> io::Result<KeyboardMode> {
        // Safety: Descriptor of VT is guaranteed to be a console
        unsafe { ioctl::ioctl(&self.descriptor, IoGetKeyboardMode) }
    }

    fn set_keyboard_mode(&self, mode: KeyboardMode) -> io::Result<()> {
        // Safety: Descriptor of VT is guaranteed to be a console
        unsafe { ioctl::ioctl(&self.descriptor, IoSetKeyboardMode::to(mode)) }
    }
}
