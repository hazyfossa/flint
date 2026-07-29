#[cfg(feature = "logind")]
mod logind;

#[cfg(feature = "seatd")]
mod seatd;

#[cfg(not(any(feature = "seatd", feature = "logind")))]
fn misconfiguration() {
    compile_error!("Select either 'logind' or 'seatd' (or both) as a supported backend")
}

use super::SessionID;
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

#[dyn_trait]
pub trait SeatManager {
    fn libseat_backend() -> &'static LibseatBackend;

    async fn list_seats(&mut self) -> Vec<SeatID>;
    async fn next_event(&mut self) -> Option<(SeatID, SeatEvent)>;
    async fn swtich(&mut self, seat: SeatID, session: SessionID);
}

pub type SeatManagerObject = Box<dyn DynSeatManager>;
pub async fn seat_manager() -> Result<SeatManagerObject> {
    // #[cfg(feature = "seatd")]
    todo!()
}
