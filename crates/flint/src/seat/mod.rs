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
use tokio::sync::broadcast;

define_env!(pub SeatID(String) = "XDG_SEAT");

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

enum SeatEvent {
    Add,
    Remove,
}

// NOTE: DBus support would require trait redesign
// to associated type Handle which implements RawSessionHandle trait
// the varlink/seatd implementations will then have to use polyfills

#[dyn_trait]
trait SeatManager {
    async fn next_event(&mut self) -> Option<(SeatID, SeatEvent)>;
    async fn swtich(&mut self, seat: SeatID, session: SessionID);
}

fn seat_manager() -> Result<Box<dyn DynSeatManager>> {
    // #[cfg(feature = "seatd")]
    todo!()
}

struct SeatHandle {
    id: SeatID,
    shutdown: broadcast::Receiver<()>,
}
