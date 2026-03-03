pub mod daemon;
mod environment;
mod worker;

use crate::bind::tty::VtNumber;
pub use worker::SessionContext;

use environment::Seat;

pub struct View {
    pub seat: Seat,
    pub vt: VtNumber,
}
