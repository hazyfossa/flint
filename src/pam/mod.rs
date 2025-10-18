#![allow(dead_code)]
mod converse;
pub use converse::PamDisplay;

use anyhow::{Result, anyhow, bail};
use libc::c_void;
use pam_sys::{PamConversation, PamFlag, PamHandle, PamItemType, PamReturnCode};

use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr,
};

use crate::environment::Env;

pub struct PAM<'a> {
    handle: &'a mut PamHandle,
    last_code: PamReturnCode,

    _conversation: PamConversation,
}

// TODO: simplify the most common case of immediate return (of ret)
macro_rules! pam_call {
    (let $ret:ident = $self:ident.$method:ident( $($args:tt)* )) => {
        $self.last_code = pam_sys::$method($self.handle, $($args)* );

        let $ret = match $self.last_code {
            PamReturnCode::SUCCESS => Ok(()),
            err => Err(anyhow!("pam error at `{}`: {err}", stringify!($method))),
        };
    };
}

impl PAM<'_> {
    pub fn new(
        service_name: &str,
        display: impl PamDisplay,
        single_user: Option<&str>,
    ) -> Result<Self> {
        let handle: *mut PamHandle = ptr::null_mut();

        let conversation = converse::PamConversationHandler::with_display(display).pass_to_pam();

        match pam_sys::start(service_name, single_user, &conversation, handle as _) {
            PamReturnCode::SUCCESS => Ok(Self {
                _conversation: conversation,
                // Safety: PAM is expected to fill the handle after we call start
                handle: unsafe { &mut *handle },

                last_code: PamReturnCode::SUCCESS,
            }),
            err => bail!(err),
        }
    }

    pub fn authenticate(&mut self, flags: PamFlag) -> Result<()> {
        pam_call!(let ret = self.authenticate(flags));
        ret
    }

    pub fn acct_mgmt(&mut self, flags: PamFlag) -> Result<()> {
        pam_call!(let ret = self.acct_mgmt(flags));
        ret
    }

    pub fn setcred(&mut self, flags: PamFlag) -> Result<()> {
        pam_call!(let ret = self.setcred(flags));
        ret
    }

    pub fn open_session(&mut self, flags: PamFlag) -> Result<()> {
        pam_call!(let ret = self.open_session(flags));
        ret
    }

    pub fn close_session(&mut self, flags: PamFlag) -> Result<()> {
        pam_call!(let ret = self.close_session(flags));
        ret
    }

    pub fn putenv(&mut self, kv_pair: &str) -> Result<()> {
        pam_call!(let ret = self.putenv(kv_pair));
        ret
    }

    pub fn set_item(&mut self, item: PamItemType, value: &str) -> Result<()> {
        let s = CString::new(value).unwrap();
        self.last_code = PamReturnCode::from(unsafe {
            // pam_set_item is exposed in a weird way in pam_sys::wrapped, so
            // we use the raw version here instead
            pam_sys::raw::pam_set_item(self.handle, item as i32, s.as_ptr() as *const c_void)
        });
        match self.last_code {
            PamReturnCode::SUCCESS => Ok(()),
            err => Err(anyhow!("pam error at `set_item`: {err}")),
        }
    }

    pub fn get_user(&mut self) -> Result<String> {
        let mut p: *const c_char = ptr::null_mut();
        pam_call!(let ret = self.get_user(&mut p, ptr::null()));
        ret.map(|_| (unsafe { CStr::from_ptr(p) }).to_str().unwrap().to_string())
    }

    pub fn get_env(&mut self) -> Result<Env> {
        todo!()
    }

    pub fn end(&mut self) -> Result<()> {
        pam_call!(let ret = self.end(self.last_code));
        ret
    }
}
