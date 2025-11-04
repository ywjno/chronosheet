use std::fmt;

#[derive(Debug)]
pub enum ChronoError {
    FileNotFound(String),
    MissingSettings(String),
    SettingsNotFirst(String, usize),
    MissingMonth(String, usize),
    MissingRest(String, usize),
    InvalidMonthFormat(String, usize),
    InvalidRestFormat(String, usize),
    InvalidTimeFormat(String, usize),
    InvalidTimeRange(String, usize),
    UnexpectedContent(String, usize),
    IoError(std::io::Error),
}

impl fmt::Display for ChronoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChronoError::FileNotFound(filename) => {
                write!(f, "Error: Cannot read file '{}'", filename)
            }
            ChronoError::MissingSettings(filename) => {
                write!(f, "Error: Missing [settings] section in '{}'", filename)
            }
            ChronoError::SettingsNotFirst(filename, line) => {
                write!(
                    f,
                    "Error: [settings] must be first section in '{}' (line {})",
                    filename, line
                )
            }
            ChronoError::MissingMonth(filename, line) => {
                write!(
                    f,
                    "Error: Missing required field 'month' in [settings] in '{}' (line {})",
                    filename, line
                )
            }
            ChronoError::MissingRest(filename, line) => {
                write!(
                    f,
                    "Error: Missing required field 'rest' in [settings] in '{}' (line {})",
                    filename, line
                )
            }
            ChronoError::InvalidMonthFormat(filename, line) => {
                write!(
                    f,
                    "Error: Invalid month format in '{}' at line {}: expected YYYYMM",
                    filename, line
                )
            }
            ChronoError::InvalidRestFormat(filename, line) => {
                write!(
                    f,
                    "Error: Invalid rest format in '{}' at line {}: expected format like 1h, 30m, or 1h30m",
                    filename, line
                )
            }
            ChronoError::InvalidTimeFormat(filename, line) => {
                write!(
                    f,
                    "Error: Invalid time format in '{}' at line {}: expected HHMM, HH:MM, or H:MM",
                    filename, line
                )
            }
            ChronoError::InvalidTimeRange(filename, line) => {
                write!(
                    f,
                    "Error: Invalid time in '{}' at line {}: HH must be 00-23, MM must be 00-59",
                    filename, line
                )
            }
            ChronoError::UnexpectedContent(filename, line) => {
                write!(
                    f,
                    "Error: Unexpected content in '{}' at line {}",
                    filename, line
                )
            }
            ChronoError::IoError(err) => {
                write!(f, "Error: IO error: {}", err)
            }
        }
    }
}

impl std::error::Error for ChronoError {}

impl From<std::io::Error> for ChronoError {
    fn from(err: std::io::Error) -> Self {
        ChronoError::IoError(err)
    }
}

pub type Result<T> = std::result::Result<T, ChronoError>;

/// Warning messages that don't stop processing
pub enum Warning {
    InvalidDate(String, u8, String, usize),
    StartAfterEnd(String, u8, usize),
    DuplicateDay(String, u8, usize),
}

impl Warning {
    pub fn print(&self) {
        match self {
            Warning::InvalidDate(person, day, month, line) => {
                eprintln!(
                    "Warning: [{}] Invalid date: day {} in month {} (line {})",
                    person, day, month, line
                );
            }
            Warning::StartAfterEnd(person, day, line) => {
                eprintln!(
                    "Warning: [{}] Start time after end time on day {} (line {}), skipping",
                    person, day, line
                );
            }
            Warning::DuplicateDay(person, day, line) => {
                eprintln!(
                    "Warning: [{}] Duplicate entry for day {} (line {}), using first occurrence",
                    person, day, line
                );
            }
        }
    }
}
