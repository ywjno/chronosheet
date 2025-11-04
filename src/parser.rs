use crate::error::{ChronoError, Result, Warning};
use crate::types::*;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Discovers all .txt files in the given directory
pub fn find_txt_files(dir: &Path) -> Result<Vec<String>> {
    let mut txt_files = Vec::new();

    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "txt" {
                    if let Some(filename) = path.file_name() {
                        if let Some(filename_str) = filename.to_str() {
                            txt_files.push(filename_str.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(txt_files)
}

/// Line with its number for error reporting
struct Line {
    number: usize,
    content: String,
}

/// Main parser for timesheet file
pub struct Parser {
    filename: String,
    lines: Vec<Line>,
    current: usize,
    warnings: Vec<Warning>,
}

impl Parser {
    pub fn new(filename: &str) -> Result<Self> {
        let content = fs::read_to_string(filename)
            .map_err(|_| ChronoError::FileNotFound(filename.to_string()))?;

        let lines: Vec<Line> = content
            .lines()
            .enumerate()
            .map(|(idx, line)| Line {
                number: idx + 1,
                content: line.trim().to_string(),
            })
            .collect();

        Ok(Self {
            filename: filename.to_string(),
            lines,
            current: 0,
            warnings: Vec::new(),
        })
    }

    pub fn parse(&mut self) -> Result<Timesheet> {
        let settings = self.parse_settings()?;
        let persons = self.parse_persons()?;

        Ok(Timesheet { settings, persons })
    }

    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    fn peek_line(&self) -> Option<&Line> {
        self.lines.get(self.current)
    }

    fn next_line(&mut self) -> Option<&Line> {
        if self.current < self.lines.len() {
            let line = &self.lines[self.current];
            self.current += 1;
            Some(line)
        } else {
            None
        }
    }

    fn skip_empty_lines(&mut self) {
        while let Some(line) = self.peek_line() {
            if line.content.is_empty() {
                self.current += 1;
            } else {
                break;
            }
        }
    }

    fn is_section_header(&self, line: &str) -> Option<String> {
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        } else {
            None
        }
    }

    fn parse_settings(&mut self) -> Result<Settings> {
        // Skip leading empty lines
        self.skip_empty_lines();

        // First non-empty line must be [settings]
        let filename = self.filename.clone();
        let line = self
            .next_line()
            .ok_or_else(|| ChronoError::MissingSettings(filename.clone()))?;

        let content = line.content.clone();
        let line_number = line.number;

        let section_name = self
            .is_section_header(&content)
            .ok_or_else(|| ChronoError::MissingSettings(filename.clone()))?;

        if section_name.to_lowercase() != "settings" {
            return Err(ChronoError::SettingsNotFirst(
                filename.clone(),
                line_number,
            ));
        }

        let mut month: Option<String> = None;
        let mut rest: Option<Duration> = None;
        let settings_line = line_number;

        // Parse settings fields
        loop {
            self.skip_empty_lines();
            let Some(line) = self.peek_line() else {
                break;
            };

            // Check if we hit another section
            if self.is_section_header(&line.content).is_some() {
                break;
            }

            let line = self.next_line().unwrap();
            let content = line.content.clone();
            let line_num = line.number;

            // Parse key=value
            if let Some((key, value)) = content.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();

                match key.as_str() {
                    "month" => {
                        month = Some(self.parse_month(&value, line_num)?);
                    }
                    "rest" => {
                        rest = Some(self.parse_rest_duration(&value, line_num)?);
                    }
                    _ => {
                        // Ignore unknown settings
                    }
                }
            }
        }

        let month = month.ok_or_else(|| ChronoError::MissingMonth(self.filename.clone(), settings_line))?;
        let rest = rest.ok_or_else(|| ChronoError::MissingRest(self.filename.clone(), settings_line))?;

        Ok(Settings { month, rest })
    }

    fn parse_month(&self, value: &str, line_num: usize) -> Result<String> {
        // Validate YYYYMM format
        if value.len() != 6 || !value.chars().all(|c| c.is_ascii_digit()) {
            return Err(ChronoError::InvalidMonthFormat(
                self.filename.clone(),
                line_num,
            ));
        }

        // Validate month range
        let month_part = &value[4..];
        let month_num: u32 = month_part.parse().unwrap();
        if !(1..=12).contains(&month_num) {
            return Err(ChronoError::InvalidMonthFormat(
                self.filename.clone(),
                line_num,
            ));
        }

        Ok(value.to_string())
    }

    fn parse_rest_duration(&self, value: &str, line_num: usize) -> Result<Duration> {
        self.parse_duration(value, line_num, true)
    }

    fn parse_deduction_duration(&self, value: &str, line_num: usize) -> Result<Duration> {
        self.parse_duration(value, line_num, false)
    }

    fn parse_duration(&self, value: &str, line_num: usize, is_rest: bool) -> Result<Duration> {
        let value = value.trim();

        // Remove leading # and optional - for deductions
        let value = if value.starts_with('#') {
            value[1..].trim_start_matches('-').trim_start_matches('+').trim()
        } else {
            value
        };

        let mut hours: Option<u32> = None;
        let mut minutes: Option<u32> = None;
        let mut remaining = value;

        // Parse hours
        if let Some(h_pos) = remaining.find('h') {
            let h_str = &remaining[..h_pos];
            hours = Some(
                h_str
                    .parse()
                    .map_err(|_| ChronoError::InvalidRestFormat(self.filename.clone(), line_num))?,
            );
            remaining = &remaining[h_pos + 1..];
        }

        // Parse minutes
        if let Some(m_pos) = remaining.find('m') {
            let m_str = &remaining[..m_pos];
            if !m_str.is_empty() {
                minutes = Some(
                    m_str
                        .parse()
                        .map_err(|_| ChronoError::InvalidRestFormat(self.filename.clone(), line_num))?,
                );
            }
            remaining = &remaining[m_pos + 1..];
        }

        // Check for invalid remaining characters or patterns
        if !remaining.trim().is_empty() || (hours.is_none() && minutes.is_none()) {
            return Err(ChronoError::InvalidRestFormat(
                self.filename.clone(),
                line_num,
            ));
        }

        let hours = hours.unwrap_or(0);
        let minutes = minutes.unwrap_or(0);

        // Validate constraints
        if is_rest && hours > 2 {
            return Err(ChronoError::InvalidRestFormat(
                self.filename.clone(),
                line_num,
            ));
        }

        if minutes > 59 {
            return Err(ChronoError::InvalidRestFormat(
                self.filename.clone(),
                line_num,
            ));
        }

        Ok(Duration::new(hours, minutes))
    }

    fn parse_persons(&mut self) -> Result<Vec<PersonRecord>> {
        let mut persons = Vec::new();

        loop {
            self.skip_empty_lines();
            let Some(line) = self.peek_line() else {
                break;
            };

            if let Some(name) = self.is_section_header(&line.content) {
                self.next_line(); // consume section header
                let person = self.parse_person(name)?;
                persons.push(person);
            } else {
                break;
            }
        }

        Ok(persons)
    }

    fn parse_person(&mut self, name: String) -> Result<PersonRecord> {
        let mut person = PersonRecord::new(name);
        let mut seen_days: HashSet<u8> = HashSet::new();

        loop {
            self.skip_empty_lines();
            let Some(line) = self.peek_line() else {
                break;
            };

            // Check if we hit another section
            if self.is_section_header(&line.content).is_some() {
                break;
            }

            // Try to parse as day number (but check it's not a time format first)
            // Valid day numbers are 1-31, and shouldn't look like time (contain ':' or be 4 digits)
            let is_time_format = line.content.contains(':') ||
                                 (line.content.len() == 4 && line.content.chars().all(|c| c.is_ascii_digit()));

            if !is_time_format && line.content.parse::<u8>().is_ok() {
                let day = line.content.parse::<u8>().unwrap();
                let day_line = line.number;
                self.next_line(); // consume day line

                // Check for duplicate
                if seen_days.contains(&day) {
                    self.warnings.push(Warning::DuplicateDay(
                        person.name.clone(),
                        day,
                        day_line,
                    ));
                    // Skip this record - consume until next day or section
                    self.skip_current_day_record()?;
                    continue;
                }
                seen_days.insert(day);

                let mut record = DayRecord::new(day, day_line);

                // Parse start time (required, but can be missing)
                if let Some(start_line) = self.peek_line() {
                    let looks_like_time = start_line.content.contains(':') ||
                                          (start_line.content.len() == 4 && start_line.content.chars().all(|c| c.is_ascii_digit()));
                    let looks_like_day = !looks_like_time && start_line.content.parse::<u8>().is_ok() && start_line.content.len() <= 2;

                    if !start_line.content.is_empty()
                        && self.is_section_header(&start_line.content).is_none()
                        && !looks_like_day
                        && !start_line.content.starts_with('#')
                    {
                        // This should be a time
                        record.start = Some(self.parse_time(&start_line.content, start_line.number)?);
                        self.next_line();
                    }
                }

                // Parse end time (required, but can be missing)
                if let Some(end_line) = self.peek_line() {
                    let looks_like_time = end_line.content.contains(':') ||
                                          (end_line.content.len() == 4 && end_line.content.chars().all(|c| c.is_ascii_digit()));
                    let looks_like_day = !looks_like_time && end_line.content.parse::<u8>().is_ok() && end_line.content.len() <= 2;

                    if !end_line.content.is_empty()
                        && self.is_section_header(&end_line.content).is_none()
                        && !looks_like_day
                        && !end_line.content.starts_with('#')
                    {
                        // This should be a time
                        record.end = Some(self.parse_time(&end_line.content, end_line.number)?);
                        self.next_line();
                    }
                }

                // Parse optional deduction
                if let Some(ded_line) = self.peek_line() {
                    if ded_line.content.starts_with('#') {
                        record.deduction = self.parse_deduction_duration(&ded_line.content, ded_line.number)?;
                        self.next_line();
                    }
                }

                person.days.push(record);
            } else {
                // Unexpected content
                let line = self.next_line().unwrap();
                let line_num = line.number;
                let filename = self.filename.clone();
                return Err(ChronoError::UnexpectedContent(
                    filename,
                    line_num,
                ));
            }
        }

        Ok(person)
    }

    fn skip_current_day_record(&mut self) -> Result<()> {
        // Skip up to 3 more lines (start, end, deduction)
        for _ in 0..3 {
            if let Some(line) = self.peek_line() {
                if line.content.is_empty()
                    || self.is_section_header(&line.content).is_some()
                    || line.content.parse::<u8>().is_ok()
                {
                    break;
                }
                self.next_line();
            } else {
                break;
            }
        }
        Ok(())
    }

    fn parse_time(&self, value: &str, line_num: usize) -> Result<Time> {
        let value = value.trim();

        // Try format 1: 4-digit without colon (e.g., "0815")
        if !value.contains(':') {
            if value.len() == 4 && value.chars().all(|c| c.is_ascii_digit()) {
                let hour: u8 = value[0..2].parse().map_err(|_| {
                    ChronoError::InvalidTimeFormat(self.filename.clone(), line_num)
                })?;
                let minute: u8 = value[2..4].parse().map_err(|_| {
                    ChronoError::InvalidTimeFormat(self.filename.clone(), line_num)
                })?;
                return Time::new(hour, minute).ok_or_else(|| {
                    ChronoError::InvalidTimeRange(self.filename.clone(), line_num)
                });
            } else {
                return Err(ChronoError::InvalidTimeFormat(
                    self.filename.clone(),
                    line_num,
                ));
            }
        }

        // Try format 2 & 3: with colon (e.g., "08:15" or "8:15")
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() != 2 {
            return Err(ChronoError::InvalidTimeFormat(
                self.filename.clone(),
                line_num,
            ));
        }

        // Hour can be 1 or 2 digits, minute must be 2 digits
        if parts[0].is_empty() || parts[0].len() > 2 || parts[1].len() != 2 {
            return Err(ChronoError::InvalidTimeFormat(
                self.filename.clone(),
                line_num,
            ));
        }

        let hour: u8 = parts[0].parse().map_err(|_| {
            ChronoError::InvalidTimeFormat(self.filename.clone(), line_num)
        })?;

        let minute: u8 = parts[1].parse().map_err(|_| {
            ChronoError::InvalidTimeFormat(self.filename.clone(), line_num)
        })?;

        Time::new(hour, minute).ok_or_else(|| {
            ChronoError::InvalidTimeRange(self.filename.clone(), line_num)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_parser(content: &str) -> Parser {
        let lines: Vec<Line> = content
            .lines()
            .enumerate()
            .map(|(idx, line)| Line {
                number: idx + 1,
                content: line.trim().to_string(),
            })
            .collect();

        Parser {
            filename: "test.txt".to_string(),
            lines,
            current: 0,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn test_parse_time_formats() {
        struct TestCase {
            input: &'static str,
            expected_hour: u8,
            expected_minute: u8,
            should_succeed: bool,
        }

        let test_cases = vec![
            // Valid formats
            TestCase { input: "0815", expected_hour: 8, expected_minute: 15, should_succeed: true },
            TestCase { input: "08:15", expected_hour: 8, expected_minute: 15, should_succeed: true },
            TestCase { input: "8:15", expected_hour: 8, expected_minute: 15, should_succeed: true },
            TestCase { input: "00:00", expected_hour: 0, expected_minute: 0, should_succeed: true },
            TestCase { input: "23:59", expected_hour: 23, expected_minute: 59, should_succeed: true },
            TestCase { input: "2359", expected_hour: 23, expected_minute: 59, should_succeed: true },
            TestCase { input: "0000", expected_hour: 0, expected_minute: 0, should_succeed: true },
            TestCase { input: "12:00", expected_hour: 12, expected_minute: 0, should_succeed: true },
            TestCase { input: "1:30", expected_hour: 1, expected_minute: 30, should_succeed: true },

            // Invalid formats
            TestCase { input: "25:00", expected_hour: 0, expected_minute: 0, should_succeed: false }, // hour > 23
            TestCase { input: "08:60", expected_hour: 0, expected_minute: 0, should_succeed: false }, // minute > 59
            TestCase { input: "24:00", expected_hour: 0, expected_minute: 0, should_succeed: false }, // hour > 23
            TestCase { input: "8:5", expected_hour: 0, expected_minute: 0, should_succeed: false },   // minute not 2 digits
            TestCase { input: "815", expected_hour: 0, expected_minute: 0, should_succeed: false },   // only 3 digits
            TestCase { input: "08:15:00", expected_hour: 0, expected_minute: 0, should_succeed: false }, // too many colons
            TestCase { input: "abc", expected_hour: 0, expected_minute: 0, should_succeed: false },   // not a number
            TestCase { input: "08:", expected_hour: 0, expected_minute: 0, should_succeed: false },   // missing minute
            TestCase { input: ":15", expected_hour: 0, expected_minute: 0, should_succeed: false },   // missing hour
        ];

        let parser = create_test_parser("");

        for (i, tc) in test_cases.iter().enumerate() {
            let result = parser.parse_time(tc.input, 1);

            if tc.should_succeed {
                assert!(
                    result.is_ok(),
                    "Test case {} failed: '{}' should succeed but got error: {:?}",
                    i, tc.input, result.err()
                );

                let time = result.unwrap();
                assert_eq!(
                    time.hour(), tc.expected_hour,
                    "Test case {}: '{}' hour mismatch",
                    i, tc.input
                );
                assert_eq!(
                    time.minute(), tc.expected_minute,
                    "Test case {}: '{}' minute mismatch",
                    i, tc.input
                );
            } else {
                assert!(
                    result.is_err(),
                    "Test case {} failed: '{}' should fail but succeeded",
                    i, tc.input
                );
            }
        }
    }

    #[test]
    fn test_parse_duration_rest() {
        struct TestCase {
            input: &'static str,
            expected_hours: u32,
            expected_minutes: u32,
            should_succeed: bool,
        }

        let test_cases = vec![
            // Valid rest formats
            TestCase { input: "1h", expected_hours: 1, expected_minutes: 0, should_succeed: true },
            TestCase { input: "30m", expected_hours: 0, expected_minutes: 30, should_succeed: true },
            TestCase { input: "1h30m", expected_hours: 1, expected_minutes: 30, should_succeed: true },
            TestCase { input: "0h", expected_hours: 0, expected_minutes: 0, should_succeed: true },
            TestCase { input: "2h", expected_hours: 2, expected_minutes: 0, should_succeed: true },
            TestCase { input: "0h0m", expected_hours: 0, expected_minutes: 0, should_succeed: true },
            TestCase { input: "2h59m", expected_hours: 2, expected_minutes: 59, should_succeed: true },
            TestCase { input: "1h0m", expected_hours: 1, expected_minutes: 0, should_succeed: true },

            // Invalid formats
            TestCase { input: "3h", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // hours > 2 for rest
            TestCase { input: "1h60m", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // minutes > 59
            TestCase { input: "1h1h", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // duplicate h
            TestCase { input: "15m1h", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // wrong order
            TestCase { input: "15m15m", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // duplicate m
            TestCase { input: "1h120m", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // minutes > 59
            TestCase { input: "abc", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // not a duration
            TestCase { input: "", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // empty
        ];

        let parser = create_test_parser("");

        for (i, tc) in test_cases.iter().enumerate() {
            let result = parser.parse_rest_duration(tc.input, 1);

            if tc.should_succeed {
                assert!(
                    result.is_ok(),
                    "Test case {} failed: '{}' should succeed but got error: {:?}",
                    i, tc.input, result.err()
                );

                let duration = result.unwrap();
                let total_minutes = duration.minutes();
                let expected_total = (tc.expected_hours * 60 + tc.expected_minutes) as i32;
                assert_eq!(
                    total_minutes, expected_total,
                    "Test case {}: '{}' duration mismatch (got {}min, expected {}min)",
                    i, tc.input, total_minutes, expected_total
                );
            } else {
                assert!(
                    result.is_err(),
                    "Test case {} failed: '{}' should fail but succeeded with {:?}",
                    i, tc.input, result.ok()
                );
            }
        }
    }

    #[test]
    fn test_parse_duration_deduction() {
        struct TestCase {
            input: &'static str,
            expected_hours: u32,
            expected_minutes: u32,
            should_succeed: bool,
        }

        let test_cases = vec![
            // Valid deduction formats
            TestCase { input: "#-1h", expected_hours: 1, expected_minutes: 0, should_succeed: true },
            TestCase { input: "#1h", expected_hours: 1, expected_minutes: 0, should_succeed: true },
            TestCase { input: "#-30m", expected_hours: 0, expected_minutes: 30, should_succeed: true },
            TestCase { input: "#30m", expected_hours: 0, expected_minutes: 30, should_succeed: true },
            TestCase { input: "#-1h30m", expected_hours: 1, expected_minutes: 30, should_succeed: true },
            TestCase { input: "#1h30m", expected_hours: 1, expected_minutes: 30, should_succeed: true },
            TestCase { input: "#+1h", expected_hours: 1, expected_minutes: 0, should_succeed: true }, // + is stripped
            TestCase { input: "#5h", expected_hours: 5, expected_minutes: 0, should_succeed: true }, // no hour limit for deduction

            // Invalid formats
            TestCase { input: "#1h60m", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // minutes > 59
            TestCase { input: "#abc", expected_hours: 0, expected_minutes: 0, should_succeed: false }, // not a duration
        ];

        let parser = create_test_parser("");

        for (i, tc) in test_cases.iter().enumerate() {
            let result = parser.parse_deduction_duration(tc.input, 1);

            if tc.should_succeed {
                assert!(
                    result.is_ok(),
                    "Test case {} failed: '{}' should succeed but got error: {:?}",
                    i, tc.input, result.err()
                );

                let duration = result.unwrap();
                let total_minutes = duration.minutes();
                let expected_total = (tc.expected_hours * 60 + tc.expected_minutes) as i32;
                assert_eq!(
                    total_minutes, expected_total,
                    "Test case {}: '{}' duration mismatch",
                    i, tc.input
                );
            } else {
                assert!(
                    result.is_err(),
                    "Test case {} failed: '{}' should fail but succeeded",
                    i, tc.input
                );
            }
        }
    }

    #[test]
    fn test_parse_month() {
        struct TestCase {
            input: &'static str,
            should_succeed: bool,
        }

        let test_cases = vec![
            // Valid months
            TestCase { input: "202511", should_succeed: true },
            TestCase { input: "202501", should_succeed: true },
            TestCase { input: "202512", should_succeed: true },
            TestCase { input: "202402", should_succeed: true }, // leap year

            // Invalid formats
            TestCase { input: "20251", should_succeed: false },  // too short
            TestCase { input: "2025111", should_succeed: false }, // too long
            TestCase { input: "202500", should_succeed: false },  // month 00
            TestCase { input: "202513", should_succeed: false },  // month 13
            TestCase { input: "20251a", should_succeed: false },  // not all digits
            TestCase { input: "abcdef", should_succeed: false },  // not a number
        ];

        let parser = create_test_parser("");

        for (i, tc) in test_cases.iter().enumerate() {
            let result = parser.parse_month(tc.input, 1);

            if tc.should_succeed {
                assert!(
                    result.is_ok(),
                    "Test case {} failed: '{}' should succeed but got error: {:?}",
                    i, tc.input, result.err()
                );
                assert_eq!(result.unwrap(), tc.input);
            } else {
                assert!(
                    result.is_err(),
                    "Test case {} failed: '{}' should fail but succeeded",
                    i, tc.input
                );
            }
        }
    }

    #[test]
    fn test_is_section_header() {
        struct TestCase {
            input: &'static str,
            expected: Option<&'static str>,
        }

        let test_cases = vec![
            TestCase { input: "[settings]", expected: Some("settings") },
            TestCase { input: "[Tom]", expected: Some("Tom") },
            TestCase { input: "[John Doe]", expected: Some("John Doe") },
            TestCase { input: "[Alice_123]", expected: Some("Alice_123") },
            TestCase { input: "[settings", expected: None },  // missing closing bracket
            TestCase { input: "settings]", expected: None },  // missing opening bracket
            TestCase { input: "[]", expected: None },         // empty section
            TestCase { input: "Tom", expected: None },        // not a section
            TestCase { input: "08:15", expected: None },      // time format
        ];

        let parser = create_test_parser("");

        for (i, tc) in test_cases.iter().enumerate() {
            let result = parser.is_section_header(tc.input);

            assert_eq!(
                result.as_deref(), tc.expected,
                "Test case {} failed: '{}' expected {:?} but got {:?}",
                i, tc.input, tc.expected, result
            );
        }
    }

    #[test]
    fn test_parse_complete_document_basic() {
        let content = r#"
[settings]
month=202511
rest=1h

[Tom]
1
08:00
17:00
"#;

        let mut parser = create_test_parser(content);
        let result = parser.parse();

        assert!(result.is_ok(), "Parse should succeed but got: {:?}", result.err());

        let timesheet = result.unwrap();
        assert_eq!(timesheet.settings.month, "202511");
        assert_eq!(timesheet.settings.rest.minutes(), 60);
        assert_eq!(timesheet.persons.len(), 1);
        assert_eq!(timesheet.persons[0].name, "Tom");
        assert_eq!(timesheet.persons[0].days.len(), 1);
        assert_eq!(timesheet.persons[0].days[0].day, 1);
        assert_eq!(timesheet.persons[0].days[0].start.unwrap().hour(), 8);
        assert_eq!(timesheet.persons[0].days[0].end.unwrap().hour(), 17);
    }

    #[test]
    fn test_parse_complete_document_multiple_people() {
        let content = r#"
[settings]
month=202511
rest=1h

[Tom]
1
0815
1930
#-1h

[John]
2
08:15
17:30
"#;

        let mut parser = create_test_parser(content);
        let result = parser.parse();

        assert!(result.is_ok(), "Parse should succeed");

        let timesheet = result.unwrap();
        assert_eq!(timesheet.persons.len(), 2);
        assert_eq!(timesheet.persons[0].name, "Tom");
        assert_eq!(timesheet.persons[1].name, "John");

        // Check Tom's deduction
        assert_eq!(timesheet.persons[0].days[0].deduction.minutes(), 60);
    }

    #[test]
    fn test_parse_complete_document_multiple_days() {
        let content = r#"
[settings]
month=202511
rest=1h

[Alice]
1
08:00
17:00

2
08:15
17:30

3
8:00
17:00
"#;

        let mut parser = create_test_parser(content);
        let result = parser.parse();

        assert!(result.is_ok(), "Parse should succeed");

        let timesheet = result.unwrap();
        assert_eq!(timesheet.persons.len(), 1);
        assert_eq!(timesheet.persons[0].days.len(), 3);
        assert_eq!(timesheet.persons[0].days[0].day, 1);
        assert_eq!(timesheet.persons[0].days[1].day, 2);
        assert_eq!(timesheet.persons[0].days[2].day, 3);
    }

    #[test]
    fn test_parse_missing_settings() {
        let content = r#"
[Tom]
1
08:00
17:00
"#;

        let mut parser = create_test_parser(content);
        let result = parser.parse();

        assert!(result.is_err(), "Should fail without settings section");
    }

    #[test]
    fn test_parse_missing_month() {
        let content = r#"
[settings]
rest=1h

[Tom]
1
08:00
17:00
"#;

        let mut parser = create_test_parser(content);
        let result = parser.parse();

        assert!(result.is_err(), "Should fail without month");
    }

    #[test]
    fn test_parse_missing_rest() {
        let content = r#"
[settings]
month=202511

[Tom]
1
08:00
17:00
"#;

        let mut parser = create_test_parser(content);
        let result = parser.parse();

        assert!(result.is_err(), "Should fail without rest");
    }

    #[test]
    fn test_parse_invalid_time_in_document() {
        let content = r#"
[settings]
month=202511
rest=1h

[Tom]
1
25:00
17:00
"#;

        let mut parser = create_test_parser(content);
        let result = parser.parse();

        assert!(result.is_err(), "Should fail with invalid time 25:00");
    }

    #[test]
    fn test_parse_empty_lines_between_sections() {
        let content = r#"
[settings]
month=202511
rest=1h


[Tom]
1
08:00
17:00


[John]
2
09:00
18:00
"#;

        let mut parser = create_test_parser(content);
        let result = parser.parse();

        assert!(result.is_ok(), "Should handle empty lines between sections");
        let timesheet = result.unwrap();
        assert_eq!(timesheet.persons.len(), 2);
    }
}
