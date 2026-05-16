mod converse;
pub use converse::PamDisplay;

mod types;
use rustix::path::Arg;
pub use types::{CredentialsOP, PamItemType};
use types::{FlagsBuilder, flags};

use anyhow::{Context, Result, anyhow, bail};
use pam_sys::{PamConversation, PamHandle, PamReturnCode, raw as sys};

use std::{
    ffi::{CStr, CString, c_void},
    os::raw::c_char,
    ptr,
};

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
            unsafe { sys::$method($self.handle, $($args)* ) }
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

        let conversation = converse::PamConversationHandler::with_display(display).build();

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
        let mut user: *const c_char = ptr::null_mut(); // TODO: is this correct?
        let prompt: *const c_char = ptr::null();

        pam_call!(let ret = self.pam_get_user(&mut user, prompt));
        ret?;

        let user = unsafe { CStr::from_ptr(user) };

        user.to_str()
            .context("Username was not valid UTF-8")
            .map(|s| s.to_string())
    }

    pub fn set_env(&mut self, env: impl envy::diff::Diff) -> Result<()> {
        // NOTE: misc_paste_env in pam_sys::wrappped is constrained to unicode (UTF-8)
        // while our Env (and this impl) is not

        let env = env_to_c_pointers(env);
        pam_call!(let ret = self.pam_misc_paste_env(env.as_ptr()));
        ret
    }

    pub fn end(mut self) -> Result<()> {
        pam_call!(let ret = self.pam_end(self.last_code as i32));
        ret
    }
}

// impl EnvContainer for PAM<'_> {
//     fn raw_get(&self, key: &str) -> Option<std::ffi::OsString> {
//         let cstr = CStr::from(key);
//         let ret = sys::
//     }
// }

fn env_to_c_pointers(env: impl envy::diff::Diff) -> Vec<*const i8> {
    let env_vec: Vec<_> = env
        .to_env_diff()
        .into_iter()
        // TODO: do not unwrap
        .map(|env_pair| env_pair.to_os_string().into_c_str().unwrap())
        .collect();

    env_vec
        .iter()
        .map(|env| env.as_ptr())
        .chain(Some(ptr::null()))
        .collect()
}
