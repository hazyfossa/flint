#[cfg(feature = "logind")]
mod logind;

#[cfg(feature = "seatd")]
mod seatd;

#[cfg(not(any(feature = "seatd", feature = "logind")))]
fn misconfiguration() {
    compile_error!("Select either 'logind' or 'seatd' (or both) as a supported backend")
}

use anyhow::Result;
use dyn_utils::dyn_trait;
use envy::define_env;

define_env!(pub LibseatBackend(String) = "LIBSEAT_BACKEND");

define_env!(
    #[derive(PartialEq, Eq, Hash)]
    pub SeatID(String) = "XDG_SEAT"
);

impl SeatID {
    pub fn seat0() -> Self {
        Self("seat0".to_string())
    }

    pub fn is_seat0(&self) -> bool {
        self.as_str() == "seat0"
    }
}

impl Default for SeatID {
    fn default() -> Self {
        Self::seat0()
    }
}

pub enum SeatEvent {
    Added,
    Changed,
    Removed,
}

// NOTE: DBus support would require trait redesign
// to associated type Handle which implements RawSessionHandle trait
// the varlink/seatd implementations will then have to use polyfills

pub struct SeatProperties {
    pub view: view::View,
    pub can_graphical: bool,
}

#[dyn_trait]
pub trait SeatManager {
    fn libseat_backend() -> &'static str;

    async fn list_seats(&mut self) -> Vec<SeatID>;
    async fn next_event(&mut self) -> Option<(SeatID, SeatEvent)>;

    // All this method does is enriches
    async fn query(&mut self, id: &SeatID) -> Result<SeatProperties>;
    // async fn swtich(&mut self, seat: SeatID, session: SessionID);
}

pub type SeatManagerObject = Box<dyn DynSeatManager>;
pub async fn seat_manager() -> Result<SeatManagerObject> {
    // #[cfg(feature = "seatd")]
    todo!()
}

// Views are purely flint's abstraction over seats:
// a view is functionally equivalent to a seat in every way
//
// The main difference is that they never pass "seat0" around
// as a special case, which results in (subjectively) better code
pub mod view {
    use anyhow::Context;
    use tracing::warn;

    use crate::{
        seat::SeatID,
        utils::{tty::VtNumber, warn::WarnExt},
    };

    pub enum View {
        Vt(VtNumber),
        Seat(SeatID),
    }

    impl View {
        pub fn seat(&self) -> Option<SeatID> {
            match self {
                Self::Seat(x) => Some(x.clone()),
                _ => None,
            }
        }

        pub fn from_env(env: &impl envy::Get) -> Option<Self> {
            let vt = env
                .maybe_get::<VtNumber>()
                .context("invalid vt, ignoring")
                .warn()
                .flatten();

            let seat = env
                .maybe_get::<SeatID>()
                .context("invalid seat, ignoring")
                .warn()
                .flatten();

            let seat = seat.filter(|x| !x.is_seat0());

            if let Some(x) = seat {
                if vt.is_some() {
                    warn!("Both non-zero seat and vt specified, ignoring vt");
                }

                Some(Self::Seat(x))
            } else {
                vt.map(Self::Vt)
            }
        }
    }
}
