mod logind;
mod seatd;

use envy::define_env;
use eyre::Result;
use futures_util::Stream;
use plug::plug;

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

pub struct SeatProperties {
    pub has_active_session: bool,
    pub can_graphical: bool,
}

#[plug(mod(logind))] // seatd
pub trait SeatManager {
    async fn stream_seats(
        &mut self,
    ) -> Result<impl Stream<Item = Result<(SeatID, SeatProperties)>>>;
}

// Views are flint's abstraction over seats.
// A view will transparently handle all required VT management
// allowing other code to never worry about VTs
pub mod view {
    use eyre::Context;
    use tracing::warn;

    use crate::{
        seat::SeatID,
        utils::{tty::VtNumber, warn::WarnExt},
    };

    pub enum View {
        StaticVt(VtNumber),
        Seat(SeatID),
    }

    impl View {
        pub fn seat(&self) -> SeatID {
            match self {
                Self::Seat(x) => x.clone(),
                Self::StaticVt(_) => SeatID::seat0(),
            }
        }

        // TODO: do we ever need to allocate VTs in flint
        // maybe when seat is seat0?
        // check how much of vt management logind/seatd do
        pub fn vt(&self) -> Option<VtNumber> {
            match self {
                Self::Seat(_) => None,
                Self::StaticVt(vt) => Some(vt.clone()),
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
                vt.map(Self::StaticVt)
            }
        }
    }
}
