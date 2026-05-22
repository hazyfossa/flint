// These are redefinitions of types in pam_sys::types
// that are (subjectively) easier to work with
#![allow(dead_code)]
pub use pam_sys::types::PamItemType;

type Flag = i32;
// Combined flags are just a union
type Flags = Flag;

pub mod flags {
    use super::Flag;

    /// Authentication service should not generate any messages
    pub const SILENT: Flag = 0x8000;

    /// The authentication service should return AUTH_ERROR
    /// if the user has a null authentication token
    /// (used by pam_authenticate{,_secondary}())
    pub const DISALLOW_NULL_AUTHTOK: Flag = 0x0001;

    pub(super) const ESTABLISH_CRED: Flag = 0x0002;
    pub(super) const DELETE_CRED: Flag = 0x0004;
    pub(super) const REINITIALIZE_CRED: Flag = 0x0008;
    pub(super) const REFRESH_CRED: Flag = 0x0010;

    /// The password service should only update those passwords that have aged.
    /// If this flag is not passed, the password service should update all passwords.
    /// (used by pam_chauthtok)
    pub const CHANGE_EXPIRED_AUTHTOK: Flag = 0x0020;

    pub const NONE: Flag = 0x0000;
}

pub enum CredentialsOP {
    // Initialize the credentials for the user.
    Establish,

    // Delete the user's credentials.
    Delete,

    // Fully reinitialize the user's credentials.
    Reinitialize,

    // Extend the lifetime of the existing credentials.
    Refresh,
}

impl Into<Flag> for CredentialsOP {
    fn into(self) -> Flag {
        match self {
            Self::Establish => flags::ESTABLISH_CRED,
            Self::Delete => flags::DELETE_CRED,
            Self::Reinitialize => flags::REINITIALIZE_CRED,
            Self::Refresh => flags::REFRESH_CRED,
        }
    }
}

pub struct FlagsBuilder(Flags);

impl FlagsBuilder {
    pub fn new() -> Self {
        Self(flags::NONE)
    }

    pub fn from(value: Flags) -> Self {
        Self(value)
    }

    #[inline]
    pub fn set_if(self, condition: bool, flag: Flag) -> Self {
        if condition { Self(self.0 | flag) } else { self }
    }

    pub fn finish(self) -> Flags {
        self.0
    }
}
