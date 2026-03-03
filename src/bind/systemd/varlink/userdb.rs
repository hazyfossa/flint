use serde::{Deserialize, Serialize};
use zlink::{ReplyError, proxy};

/// Proxy trait for calling methods on the interface.
#[proxy("io.systemd.UserDatabase")]
pub trait UserDatabase {
    /// Retrieve one or more user records. Look-up is either keyed by UID or user name, or if neither is specified all known records are enumerated.
    /// [Supports 'more' flag]
    async fn get_user_record(
        &mut self,
        uid: Option<i64>,
        user_name: Option<&str>,
        fuzzy_names: Option<&[&str]>,
        disposition_mask: Option<&[&str]>,
        uid_min: Option<i64>,
        uid_max: Option<i64>,
        service: &str,
    ) -> zlink::Result<Result<GetUserRecordOutput, UserDatabaseError>>;

    /// Retrieve one or more group records. Look-up is either keyed by GID or group name, or if neither is specified all known records are enumerated.
    /// [Supports 'more' flag]
    async fn get_group_record(
        &mut self,
        gid: Option<i64>,
        group_name: Option<&str>,
        fuzzy_names: Option<&[&str]>,
        disposition_mask: Option<&[&str]>,
        gid_min: Option<i64>,
        gid_max: Option<i64>,
        service: &str,
    ) -> zlink::Result<Result<GetGroupRecordOutput, UserDatabaseError>>;
    /// Retrieve membership relationships between users and groups.
    /// [Supports 'more' flag]
    async fn get_memberships(
        &mut self,
        user_name: Option<&str>,
        group_name: Option<&str>,
        service: &str,
    ) -> zlink::Result<Result<GetMembershipsOutput<'_>, UserDatabaseError>>;
}

/// Output parameters for the GetUserRecord method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetUserRecordOutput {
    pub record: String, // TODO
    pub incomplete: Option<bool>,
}

/// Output parameters for the GetGroupRecord method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetGroupRecordOutput {
    pub record: String, // TODO
    pub incomplete: Option<bool>,
}

/// Output parameters for the GetMemberships method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetMembershipsOutput<'a> {
    #[serde(borrow)]
    #[serde(rename = "userName")]
    pub user_name: &'a str,
    #[serde(borrow)]
    #[serde(rename = "groupName")]
    pub group_name: &'a str,
}

/// Errors that can occur in this interface.
#[derive(Debug, Clone, PartialEq, ReplyError)]
#[zlink(interface = "io.systemd.UserDatabase")]
pub enum UserDatabaseError {
    /// Error indicating that no matching user or group record was found.
    NoRecordFound,
    /// Error indicating that the contacted service does not implement the specified service name.
    BadService,
    /// Error indicating that the backing service currently is not operational and no answer can be provided.
    ServiceNotAvailable,
    /// Error indicating that there's a user record matching either UID/GID or the user/group name, but not both at the same time.
    ConflictingRecordFound,
    /// Error indicating that there's a user record matching the primary UID/GID or user/group, but that doesn't match the additional specified matches.
    NonMatchingRecordFound,
    /// Error indicating that retrieval of user/group records on this service is only supported if either user/group name or UID/GID are specified, but not if nothing is specified.
    EnumerationNotSupported,
}
