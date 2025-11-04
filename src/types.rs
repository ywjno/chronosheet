use std::fmt;

/// Represents a duration in minutes for easier calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration {
    minutes: i32,
}

impl Duration {
    pub fn new(hours: u32, minutes: u32) -> Self {
        Self {
            minutes: (hours * 60 + minutes) as i32,
        }
    }

    pub fn zero() -> Self {
        Self { minutes: 0 }
    }

    pub fn from_minutes(minutes: i32) -> Self {
        Self { minutes }
    }

    pub fn minutes(&self) -> i32 {
        self.minutes
    }

    pub fn is_negative(&self) -> bool {
        self.minutes < 0
    }
}

impl std::ops::Sub for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Duration {
        Duration {
            minutes: self.minutes - rhs.minutes,
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}h{}m", self.minutes / 60, self.minutes % 60)
    }
}

/// Represents a time in HH:MM format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    hour: u8,
    minute: u8,
}

impl Time {
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        if hour <= 23 && minute <= 59 {
            Some(Self { hour, minute })
        } else {
            None
        }
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn minute(&self) -> u8 {
        self.minute
    }

    pub fn to_minutes(&self) -> i32 {
        (self.hour as i32 * 60) + self.minute as i32
    }

    pub fn diff(&self, start: Time) -> Duration {
        Duration::from_minutes(self.to_minutes() - start.to_minutes())
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

/// Settings from [settings] section
#[derive(Debug, Clone)]
pub struct Settings {
    pub month: String,
    pub rest: Duration,
}

/// A single day's work record
#[derive(Debug, Clone)]
pub struct DayRecord {
    pub day: u8,
    pub start: Option<Time>,
    pub end: Option<Time>,
    pub deduction: Duration,
    pub line_number: usize, // Line number where this day record starts
}

impl DayRecord {
    pub fn new(day: u8, line_number: usize) -> Self {
        Self {
            day,
            start: None,
            end: None,
            deduction: Duration::zero(),
            line_number,
        }
    }
}

/// A person's complete work records
#[derive(Debug, Clone)]
pub struct PersonRecord {
    pub name: String,
    pub days: Vec<DayRecord>,
}

impl PersonRecord {
    pub fn new(name: String) -> Self {
        Self {
            name,
            days: Vec::new(),
        }
    }
}

/// Complete timesheet data
#[derive(Debug)]
pub struct Timesheet {
    pub settings: Settings,
    pub persons: Vec<PersonRecord>,
}
