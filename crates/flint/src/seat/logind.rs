mod varlink {
    use serde::{Deserialize, Serialize};
    use zlink::{ReplyError, proxy};

    /// Proxy trait for calling methods on the interface.
    #[proxy("io.systemd.Login")]
    pub trait Login {
        /// Lists current seats. If an Id filter is provided, returns the single matching seat; otherwise streams all current seats (requires the 'more' flag).
        /// [Supports 'more' flag]
        async fn list_seats(
            &mut self,
            #[zlink(rename = "Id")] id: Option<&str>,
        ) -> zlink::Result<Result<ListSeatsOutput, LoginError>>;
    }

    /// Output parameters for the ListSeats method.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct ListSeatsOutput {
        pub context: SeatContext,
        pub runtime: SeatRuntime,
    }

    /// Dual timestamp
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Timestamp {
        /// Timestamp in µs in the CLOCK_REALTIME clock (wallclock)
        pub realtime: Option<i64>,
        /// Timestamp in µs in the CLOCK_MONOTONIC clock
        pub monotonic: Option<i64>,
    }

    /// Configuration aspects of a seat
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct SeatContext {
        /// The seat identifier
        #[serde(rename = "Id")]
        pub id: String,
    }

    /// Runtime state and dynamic information of a seat
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct SeatRuntime {
        /// The currently active session on this seat, if any
        #[serde(rename = "ActiveSession")]
        pub active_session: Option<String>,
        /// Identifiers of sessions assigned to this seat
        #[serde(rename = "Sessions")]
        pub sessions: Option<Vec<String>>,
        /// Whether this seat supports text terminal sessions
        #[serde(rename = "CanTTY")]
        pub can_tty: bool,
        /// Whether this seat supports graphical sessions
        #[serde(rename = "CanGraphical")]
        pub can_graphical: bool,
        /// Whether the seat is idle
        #[serde(rename = "IdleHint")]
        pub idle_hint: bool,
        /// Timestamp when the seat went idle, only present when IdleHint is true
        #[serde(rename = "IdleSinceHint")]
        pub idle_since_hint: Option<Timestamp>,
    }

    /// Errors that can occur in this interface.
    #[derive(Debug, Clone, PartialEq, ReplyError)]
    #[zlink(interface = "io.systemd.Login")]
    pub enum LoginError {
        /// No session by this name found
        NoSuchSession,
        /// No seat by this name found
        NoSuchSeat,
        /// No user by this UID found
        NoSuchUser,
        /// No inhibitor found
        NoSuchInhibitor,
        /// Process already member of a session
        AlreadySessionMember,
        /// The specified virtual terminal (VT) is already taken by another session
        VirtualTerminalAlreadyTaken,
        /// Maximum number of sessions reached
        TooManySessions,
        /// Failed to allocate a unit for the session
        UnitAllocationFailed,
        /// The session leader process does not have a pidfd
        NoSessionPidfd,
    }
}
