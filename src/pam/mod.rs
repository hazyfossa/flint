#![allow(dead_code)]
mod converse;
pub use converse::PamDisplay;

use anyhow::{Result, anyhow, bail};
use libc::c_void;
use pam_sys::{PamConversation, PamFlag, PamHandle, PamItemType, PamReturnCode};

use std::{
    ffi::{CStr, CString},
    os::{raw::c_char, unix::ffi::OsStringExt},
    ptr,
};

use crate::environment::{Env, EnvRecipient};

pub struct PAM<'a> {
    handle: &'a mut PamHandle,
    last_code: PamReturnCode,

    _conversation: PamConversation,
}

// TODO: simplify the most common case of immediate return (of ret)
macro_rules! pam_call {
    (let $ret:ident = $self:ident.$method:ident( $($args:tt)* )) => {
        let code = pam_sys::$method($self.handle, $($args)* );
        let $ret = $self.handle_ret(code, stringify!($method));
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

    fn handle_ret(&mut self, ret: PamReturnCode, fn_name: &str) -> Result<()> {
        self.last_code = ret;
        match self.last_code {
            PamReturnCode::SUCCESS => Ok(()),
            err => Err(anyhow!("pam error at `{fn_name}`: {err}")),
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
        // NOTE: we will need to either discard everything non-unicode
        // or write a custom parser on a CStr
        // or find a lossless way from CStr to OsStr and copy the one from std
        todo!()
    }
}

impl Drop for PAM<'_> {
    fn drop(&mut self) {
        pam_call!(let _whatever = self.end(self.last_code));
    }
}

impl EnvRecipient for PAM<'_> {
    fn set_env(&mut self, env: Env) -> Result<()> {
        // NOTE: misc_paste_env in pam_sys is constrained to unicode (UTF-8)
        // while our Env is not

        let env_vec: Vec<_> = env
            .to_vec()
            .into_iter()
            .map(|env_pair| CString::new(env_pair.into_vec()).unwrap())
            .collect();

        let env_ptrs: Vec<_> = env_vec
            .iter()
            .map(|env| env.as_ptr())
            .chain(Some(ptr::null()))
            .collect();

        let ret = unsafe {
            From::from(pam_sys::raw::pam_misc_paste_env(
                self.handle as _,
                env_ptrs.as_ptr(),
            ))
        };

        self.handle_ret(ret, "set_env")
    }
}
