//! Validated geometry, representation, exact clocks and independent state storage.
mod clock;
mod epoch;

pub use clock::TickClock;
pub use epoch::Epoch;
mod geometry;
mod layout;
pub use geometry::Domain;
pub use layout::Layout;
mod resources;
mod spectrum;
mod state;
pub use resources::{ExtraStorage, ResourcePlan};
pub use spectrum::validate_spectrum;
pub use state::SpectralState;
