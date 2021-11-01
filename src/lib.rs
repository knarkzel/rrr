pub mod state;
pub mod style;

// Convenience
pub use itertools::Itertools;

// Error handling
pub use anyhow::bail;
pub use fehler::throws;
pub type Error = anyhow::Error;

/// Edits file in "EDITOR".
pub mod edit {
    use super::*;
    use std::process::{Command, Stdio};

    #[throws]
    pub fn file<P: AsRef<std::path::Path>>(file: P) {
        let editor = std::env::var("EDITOR")?;
        Command::new(&editor)
            .arg(file.as_ref())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;
    }
}
