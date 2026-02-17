use std::fmt::Display;

/// Internal struct.
pub struct Output {
    /// Internal field.
    quiet: bool,
    /// Internal field.
    verbose: bool,
}

impl Output {
    /// Internal associated function.
    pub const fn new(quiet: bool, verbose: bool) -> Self {
        Self { quiet, verbose }
    }

    /// Internal helper method.
    pub const fn quiet(&self) -> bool {
        self.quiet
    }

    /// Internal helper method.
    pub const fn verbose(&self) -> bool {
        self.verbose
    }

    /// Internal helper method.
    pub fn info(&self, msg: impl Display) {
        if !self.quiet {
            println!("{msg}");
        }
    }

    /// Internal helper method.
    pub fn warn(&self, msg: impl Display) {
        if !self.quiet {
            eprintln!("WARN: {msg}");
        }
    }

    /// Internal associated function.
    pub fn error(msg: impl Display) {
        eprintln!("{msg}");
    }
}
