use std::fmt::Display;

pub struct Output {
    quiet: bool,
    verbose: bool,
}

impl Output {
    pub const fn new(quiet: bool, verbose: bool) -> Self {
        Self { quiet, verbose }
    }

    pub const fn quiet(&self) -> bool {
        self.quiet
    }

    pub const fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn info(&self, msg: impl Display) {
        if !self.quiet {
            println!("{msg}");
        }
    }

    pub fn warn(&self, msg: impl Display) {
        if !self.quiet {
            eprintln!("WARN: {msg}");
        }
    }

    pub fn error(msg: impl Display) {
        eprintln!("{msg}");
    }
}
