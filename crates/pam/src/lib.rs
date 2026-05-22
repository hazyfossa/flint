mod converse;
use anyhow::{Context, anyhow};
pub use converse::PamDisplay;

mod types;
pub use types::{CredentialsOP, PamItemType};
use types::{FlagsBuilder, flags};

use pam_sys::{PamConversation, PamHandle as RawPamHandle, PamReturnCode, raw as sys};

use std::{
    ffi::{CStr, CString, OsString, c_void},
    os::{raw::c_char, unix::ffi::OsStringExt},
    ptr,
};

pub type Error = anyhow::Error; // TODO
type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Pam {
    handle: *mut RawPamHandle,
    last_code: PamReturnCode,

    _conversation: PamConversation,

    // NOTE: i did not find any reason for this flag to be configurable per-call
    // however, that can trivially be done
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

        let $ret = $self.handle_ret(code);
    };
}

impl Pam {
    pub fn new(
        service_name: &str,
        display: impl PamDisplay,

        // If None, PAM will query for it via prompt() on PamDisplay
        username: Option<&str>,

        silent: bool,
    ) -> Result<Self> {
        let handle: *mut RawPamHandle = ptr::null_mut();

        let conversation = converse::PamConversationHandler::with_display(display).pass_to_pam();

        match pam_sys::start(service_name, username, &conversation, handle as _) {
            PamReturnCode::SUCCESS => Ok(Self {
                _conversation: conversation,

                // Safety: PAM is expected to fill the handle after we call start
                handle: unsafe { &mut *handle },

                last_code: PamReturnCode::SUCCESS,
                silent,
            }),
            err => return Err(anyhow!(err)),
        }
    }

    fn handle_ret(&mut self, ret: PamReturnCode) -> Result<()> {
        self.last_code = ret;
        match self.last_code {
            PamReturnCode::SUCCESS => Ok(()),
            err => Err(anyhow!(err)),
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
            .context("Cannot parse username")
            .map(|s| s.to_string())
    }

    pub fn set_env(&mut self, env: impl envy::diff::Diff) -> Result<()> {
        // NOTE: misc_paste_env in pam_sys::wrappped is constrained to unicode (UTF-8)
        // while our Env (and this impl) is not

        let env = env_to_c_pointers(env);
        pam_call!(let ret = self.pam_misc_paste_env(env.as_ptr()));
        ret
    }

    // Safety: same as a manual drop, the resource should not be used afterwards
    pub unsafe fn end(&mut self) -> Result<()> {
        pam_call!(let ret = self.pam_end(self.last_code as i32));
        ret
    }
}

impl Drop for Pam {
    fn drop(&mut self) {
        unsafe {
            sys::pam_end(self.handle, self.last_code as _);
        }
    }
}

// TODO: fallible raw containers for envy
impl envy::container::EnvContainer for Pam {
    fn raw_get(&self, key: &str) -> Option<std::ffi::OsString> {
        let key = CString::new(key.as_bytes()).unwrap();
        let ret = unsafe { sys::pam_getenv(self.handle, key.as_ptr()) };

        match ret.is_null() {
            true => None,
            false => {
                let ret = unsafe { CString::from_raw(ret as _) };
                Some(OsString::from_vec(ret.as_bytes().into()))
            }
        }
    }
}

fn env_to_c_pointers(env: impl envy::diff::Diff) -> Vec<*const i8> {
    let env_vec: Vec<_> = env
        .to_env_diff()
        .into_iter()
        // TODO: is this correct? check rustix::path::Arg for reference
        .map(|env_pair| {
            CStr::from_bytes_until_nul(env_pair.to_os_string().as_encoded_bytes())
                .unwrap()
                .to_owned()
        })
        .collect();

    env_vec
        .iter()
        .map(|env| env.as_ptr())
        .chain(Some(ptr::null()))
        .collect()
}
