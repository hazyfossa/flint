use std::fmt::Display;

use super::*;
use serde::Deserialize;
use thiserror::Error;
use zlink::{ReplyError, proxy};

#[proxy("io.systemd.UserDatabase")]
trait VarlinkDefinition {
    async fn get_user_record(
        &mut self,
        uid: Option<i64>,
        #[zlink(rename = "userName")] user_name: Option<&str>,
        #[zlink(rename = "fuzzyNames")] fuzzy_names: Option<&[&str]>,
        #[zlink(rename = "dispositionMask")] disposition_mask: Option<&[&str]>,
        #[zlink(rename = "uidMin")] uid_min: Option<i64>,
        #[zlink(rename = "uidMax")] uid_max: Option<i64>,
        service: &str,
    ) -> zlink::Result<Result<GetUserRecordOutput, UserDatabaseError>>;
}

// This is intentionally incomplete: we do not need the full systemd metadata
#[derive(Debug, Deserialize)]
struct UserRecord {
    uid: Uid,
    gid: Gid,
    shell: String,
    #[serde(rename = "homeDirectory")]
    home: String,
    // TODO: support status: fallback
}

#[derive(Debug, Deserialize)]
struct GetUserRecordOutput {
    record: UserRecord,
    // TODO: log this as a warning?
    // incomplete: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, ReplyError, Error)]
#[zlink(interface = "io.systemd.UserDatabase")]
pub enum UserDatabaseError {
    NoRecordFound,
    BadService,
    ServiceNotAvailable,
    ConflictingRecordFound,
    NonMatchingRecordFound,
    EnumerationNotSupported,
}

impl Display for UserDatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoRecordFound => "No matching user or group record was found.",
            Self::BadService => "The contacted service does not implement the specified service name.",
            Self::ServiceNotAvailable => "The backing service currently is not operational and no answer can be provided.",
            Self::ConflictingRecordFound => "There's a user record matching either UID/GID or the user/group name, but not both at the same time.",
            Self::NonMatchingRecordFound => "There's a user record matching the primary UID/GID or user/group, but that doesn't match the additional specified matches.",
            Self::EnumerationNotSupported => "Retrieval of user/group records on this service is only supported if either user/group name or UID/GID are specified, but not if nothing is specified.",
        })
    }
}

pub struct UserDB {
    service: &'static str,
    conn: zlink::unix::Connection,
}

impl UserDB {
    pub async fn connect() -> Result<Self, zlink::Error> {
        Self::connect_service("io.systemd.Multiplexer").await
    }

    pub async fn connect_service(service: &'static str) -> Result<Self, zlink::Error> {
        // TODO: runtime dir may not be /run
        let conn = zlink::unix::connect(format!("/run/systemd/userdb/{service}")).await?;
        Ok(Self { service, conn })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("connection error: {0}")]
    ConnectionError(#[from] zlink::Error),
    #[error(transparent)]
    UserDbError(#[from] UserDatabaseError),
}

impl UserProvider for UserDB {
    type Error = Error;
    async fn resolve(&mut self, name: &str) -> Result<Option<UserMeta>, Self::Error> {
        let ret = self
            .conn
            .get_user_record(None, Some(name), None, None, None, None, self.service)
            .await?;

        let record = match ret {
            Ok(v) => v.record,
            // TODO: there are other errors that semantically are close to "not found"
            Err(UserDatabaseError::NoRecordFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let p = record;
        Ok(Some(UserMeta {
            uid: p.uid,
            gid: p.gid,
            home: p.home.into(),
            shell: p.shell.into(),
        }))
    }
}
