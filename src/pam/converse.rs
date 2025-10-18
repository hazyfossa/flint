use libc::{c_char, c_int, c_void, calloc, free, memcpy, size_t};
use pam_sys::{PamConversation, PamMessage, PamMessageStyle, PamResponse, PamReturnCode};
use std::{error::Error, ffi::CStr, mem, pin::Pin};
use zeroize::Zeroize;

pub struct ConversationError;

impl<E: Error> From<E> for ConversationError {
    fn from(value: E) -> Self {
        // We cannot pass any error context to pam
        drop(value);
        Self
    }
}

impl Into<PamReturnCode> for ConversationError {
    fn into(self) -> PamReturnCode {
        PamReturnCode::CONV_ERR
    }
}

pub enum MessageLevel {
    Error,
    Info,
}

pub trait PamDisplay {
    fn prompt(&self, text: &str, show: bool) -> Result<String, ConversationError>;
    fn message(&self, text: &str, level: MessageLevel) -> Result<(), ConversationError>;
}

unsafe fn to_cstr(mut s: String) -> *mut c_char {
    unsafe {
        let a = calloc(1, s.len() + 1) as *mut c_char;
        if a.is_null() {
            panic!("unable to allocate C string");
        }
        memcpy(a as *mut c_void, s.as_ptr() as *const c_void, s.len());
        s.zeroize();
        return a;
    }
}

pub struct PamConversationHandler<'a> {
    pub display: Pin<Box<dyn PamDisplay + 'a>>,
}

impl<'a> PamConversationHandler<'a> {
    pub fn with_display(display: impl PamDisplay + 'a) -> Self {
        Self {
            display: Box::pin(display),
        }
    }

    fn handle(
        &self,
        message: &PamMessage,
        response_sender: &mut PamResponse,
    ) -> Result<(), ConversationError> {
        let text = unsafe { CStr::from_ptr(message.msg) }.to_str()?;

        match PamMessageStyle::from(message.msg_style) {
            PamMessageStyle::PROMPT_ECHO_ON => {
                let response = self.display.prompt(text, true)?;
                response_sender.resp = unsafe { to_cstr(response) };
                Ok(())
            }
            PamMessageStyle::PROMPT_ECHO_OFF => {
                let response = self.display.prompt(text, false)?;
                response_sender.resp = unsafe { to_cstr(response) };
                Ok(())
            }
            PamMessageStyle::ERROR_MSG => self.display.message(text, MessageLevel::Error),
            PamMessageStyle::TEXT_INFO => self.display.message(text, MessageLevel::Info),
        }
    }

    extern "C" fn converse(
        num_msg: c_int,
        msg: *mut *mut PamMessage,
        out_resp: *mut *mut PamResponse,
        appdata_ptr: *mut c_void,
    ) -> c_int {
        // allocate space for responses
        let resp = unsafe {
            calloc(num_msg as usize, mem::size_of::<PamResponse>() as size_t) as *mut PamResponse
        };
        if resp.is_null() {
            return PamReturnCode::BUF_ERR as c_int;
        }

        let wrapped_self = unsafe { &*(appdata_ptr as *const Self) };

        let mut pam_ret = PamReturnCode::SUCCESS;
        for i in 0..num_msg as isize {
            // get indexed values
            let message: &mut PamMessage = unsafe { &mut **(msg.offset(i)) };
            let response_ptr: &mut PamResponse = unsafe { &mut *(resp.offset(i)) };

            match wrapped_self.handle(message, response_ptr) {
                Err(error) => {
                    pam_ret = error.into();
                    break;
                }
                Ok(()) => (),
            }
        }

        // free allocated memory if an error occured
        if pam_ret != PamReturnCode::SUCCESS {
            // Free any strdup'd response strings
            for i in 0..num_msg as isize {
                let r: &mut PamResponse = unsafe { &mut *(resp.offset(i)) };
                if !r.resp.is_null() {
                    unsafe { free(r.resp as *mut c_void) };
                }
            }

            // Free the response array
            unsafe { free(resp as *mut c_void) };
        } else {
            unsafe { *out_resp = resp };
        }

        pam_ret as c_int
    }

    pub fn pass_to_pam(mut self) -> PamConversation {
        PamConversation {
            conv: Some(Self::converse),
            data_ptr: &mut self as *mut PamConversationHandler as *mut c_void,
        }
    }
}
