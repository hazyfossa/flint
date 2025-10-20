mod converse;
pub use converse::PamDisplay;

mod types;
pub use types::CredentialsOP;
use types::{FlagsBuilder, flags};

use anyhow::{Result, anyhow, bail};
use pam_sys::{PamConversation, PamHandle, PamItemType, PamReturnCode};

use std::{
    ffi::{CStr, CString, c_void},
    os::{raw::c_char, unix::ffi::OsStringExt},
    ptr,
};

use crate::environment::{Env, EnvRecipient};

// TODO: RAII

pub struct PAM<'a> {
    handle: &'a mut PamHandle,
    last_code: PamReturnCode,

    _conversation: PamConversation,

    silent: bool,
}

// NOTE: we are using the raw api, since the flag definitions in pam_sys::wrapped are too inflexible
// (and some stuff is broken)
// TODO: consider upstreaming?
macro_rules! pam_call {
    (let $ret:ident = $self:ident.$method:ident( $($args:tt)* )) => {
        let code = PamReturnCode::from(
            unsafe { pam_sys::raw::$method($self.handle, $($args)* ) }
        );

        let $ret = $self.handle_ret(code, stringify!($method));
    };
}

impl PAM<'_> {
    pub fn new(
        service_name: &str,
        display: impl PamDisplay,

        // If None, PAM will query for it via prompt() on PamDisplay
        username: Option<&str>,

        // NOTE: i did not find any reason for this flag to be configurable per-call
        // however, that can trivially be done
        silent: bool,
    ) -> Result<Self> {
        let handle: *mut PamHandle = ptr::null_mut();

        let conversation = converse::PamConversationHandler::with_display(display).pass_to_pam();

        match pam_sys::start(service_name, username, &conversation, handle as _) {
            PamReturnCode::SUCCESS => Ok(Self {
                _conversation: conversation,

                // Safety: PAM is expected to fill the handle after we call start
                handle: unsafe { &mut *handle },

                last_code: PamReturnCode::SUCCESS,
                silent,
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

    pub fn authenticate(&mut self, require_auth_token: bool) -> Result<()> {
        let flags = FlagsBuilder::new()
            .set_if(self.silent, flags::SILENT)
            .set_if(require_auth_token, flags::DISALLOW_NULL_AUTHTOK)
            .finish();

        pam_call!(let ret = self.pam_authenticate(flags));
        ret
    }

    pub fn assert_account_is_valid(&mut self, require_auth_token: bool) -> Result<()> {
        let flags = FlagsBuilder::new()
            .set_if(self.silent, flags::SILENT)
            .set_if(require_auth_token, flags::DISALLOW_NULL_AUTHTOK)
            .finish();

        pam_call!(let ret = self.pam_acct_mgmt(flags));
        ret
    }

    pub fn credentials(&mut self, op: CredentialsOP) -> Result<()> {
        let flags = FlagsBuilder::from(op.into())
            .set_if(self.silent, flags::SILENT)
            .finish();

        pam_call!(let ret = self.pam_setcred(flags));
        ret
    }

    pub fn open_session(&mut self) -> Result<()> {
        let flags = FlagsBuilder::new()
            .set_if(self.silent, flags::SILENT)
            .finish();

        pam_call!(let ret = self.pam_open_session(flags));
        ret
    }

    pub fn close_session(&mut self) -> Result<()> {
        let flags = FlagsBuilder::new()
            .set_if(self.silent, flags::SILENT)
            .finish();

        pam_call!(let ret = self.pam_close_session(flags));
        ret
    }

    pub fn set_item(&mut self, item: PamItemType, value: &str) -> Result<()> {
        let s = CString::new(value).unwrap();
        pam_call!(let ret = self.pam_set_item(item as i32, s.as_ptr() as *const c_void));
        ret
    }

    pub fn get_username(&mut self) -> Result<String> {
        let mut p: *const c_char = ptr::null_mut();
        pam_call!(let ret = self.pam_get_user(&mut p, ptr::null()));
        ret.map(|_| (unsafe { CStr::from_ptr(p) }).to_str().unwrap().to_string())
    }

    pub fn get_env(&mut self) -> Result<Env> {
        // NOTE: we will need to either discard everything non-unicode
        // or write a custom parser on a CStr
        // or find a lossless way from CStr to OsStr and copy the one from std
        todo!()
    }

    pub fn end(mut self) -> Result<()> {
        pam_call!(let ret = self.pam_end(self.last_code as i32));
        ret
    }
}

fn env_to_c_pointers(env: Env) -> Vec<*const i8> {
    let env_vec: Vec<_> = env
        .to_vec()
        .into_iter()
        .map(|env_pair| CString::new(env_pair.into_vec()).unwrap())
        .collect();

    env_vec
        .iter()
        .map(|env| env.as_ptr())
        .chain(Some(ptr::null()))
        .collect()
}

impl EnvRecipient for PAM<'_> {
    fn merge_env(&mut self, env: Env) -> Result<()> {
        let env = env_to_c_pointers(env);

        for kv_pair in env.to_vec() {
            pam_call!(let ret = self.pam_putenv(kv_pair));
            ret?;
        }
        Ok(())
    }

    fn set_env(&mut self, env: Env) -> Result<()> {
        // NOTE: misc_paste_env in pam_sys is constrained to unicode (UTF-8)
        // while our Env is not

        let env = env_to_c_pointers(env);
        pam_call!(let ret = self.pam_misc_paste_env(env.as_ptr()));
        ret
    }
}
