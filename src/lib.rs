pub mod state;
pub mod utils;
pub mod style;

// Convenience
pub use itertools::Itertools;

// Error handling
pub use anyhow::bail;
pub use fehler::throws;
pub type Error = anyhow::Error;
