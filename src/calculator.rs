use crate::error::Warning;
use crate::types::*;
use chrono::NaiveDate;

/// Result data for a person
#[derive(Debug)]
pub struct PersonResult {
    pub name: String,
    pub work_days: usize,
    pub work_hours: u32,
    pub work_minutes: u32,
}

/// Calculate total work hours for all persons in the timesheet
pub fn calculate_work_hours(timesheet: &Timesheet) -> (Vec<PersonResult>, Vec<Warning>) {
    let mut results = Vec::new();
    let mut warnings = Vec::new();

    for person in &timesheet.persons {
        let (total_minutes, work_days, person_warnings) =
            calculate_person_hours(person, &timesheet.settings, &timesheet.settings.month);

        // Convert total minutes to hours and minutes
        let hours = (total_minutes / 60) as u32;
        let minutes = (total_minutes % 60) as u32;

        results.push(PersonResult {
            name: person.name.clone(),
            work_days,
            work_hours: hours,
            work_minutes: minutes,
        });
        warnings.extend(person_warnings);
    }

    (results, warnings)
}

fn calculate_person_hours(
    person: &PersonRecord,
    settings: &Settings,
    month: &str,
) -> (i32, usize, Vec<Warning>) {
    let mut total_minutes = 0;
    let mut work_days = 0;
    let mut warnings = Vec::new();

    for day_record in &person.days {
        // Validate date
        if !is_valid_date(day_record.day, month) {
            warnings.push(Warning::InvalidDate(
                person.name.clone(),
                day_record.day,
                month.to_string(),
                day_record.line_number,
            ));
            continue; // Invalid dates don't count as work days
        }

        // Valid date counts as a work day
        work_days += 1;

        // Calculate hours for this day
        let day_minutes = calculate_day_hours(day_record, settings, &person.name, &mut warnings);
        total_minutes += day_minutes;
    }

    (total_minutes, work_days, warnings)
}

fn calculate_day_hours(
    record: &DayRecord,
    settings: &Settings,
    person_name: &str,
    warnings: &mut Vec<Warning>,
) -> i32 {
    // If start or end is missing, hours = 0
    let Some(start) = record.start else {
        return 0;
    };
    let Some(end) = record.end else {
        return 0;
    };

    // Calculate raw work duration
    let raw_duration = end.diff(start);

    // Check if start is after end
    if raw_duration.is_negative() {
        warnings.push(Warning::StartAfterEnd(
            person_name.to_string(),
            record.day,
            record.line_number,
        ));
        return 0;
    }

    // Apply rest if raw duration > 3 hours (180 minutes)
    let mut work_duration = raw_duration;
    if raw_duration.minutes() > 180 {
        work_duration = work_duration - settings.rest;
    }

    // Apply deduction
    work_duration = work_duration - record.deduction;

    // Clamp to 0 if negative
    if work_duration.is_negative() {
        return 0;
    }

    work_duration.minutes()
}

fn is_valid_date(day: u8, month: &str) -> bool {
    if day == 0 || day > 31 {
        return false;
    }

    // Parse month YYYYMM
    if month.len() != 6 {
        return false;
    }

    let year: i32 = match month[0..4].parse() {
        Ok(y) => y,
        Err(_) => return false,
    };

    let month_num: u32 = match month[4..6].parse() {
        Ok(m) => m,
        Err(_) => return false,
    };

    // Try to create a date with this year, month, and day
    NaiveDate::from_ymd_opt(year, month_num, day as u32).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_date() {
        assert!(is_valid_date(1, "202511"));
        assert!(is_valid_date(30, "202511"));
        assert!(!is_valid_date(31, "202511")); // November has 30 days
        assert!(!is_valid_date(32, "202511"));
        assert!(!is_valid_date(0, "202511"));
    }

    #[test]
    fn test_leap_year() {
        assert!(is_valid_date(29, "202402")); // 2024 is leap year
        assert!(!is_valid_date(29, "202302")); // 2023 is not
    }

    #[test]
    fn test_calculate_day_hours_basic() {
        let mut record = DayRecord::new(1, 1);
        record.start = Time::new(8, 0);
        record.end = Time::new(17, 0);

        let settings = Settings {
            month: "202511".to_string(),
            rest: Duration::new(1, 0),
        };

        let mut warnings = Vec::new();
        let minutes = calculate_day_hours(&record, &settings, "Test", &mut warnings);

        // 9 hours - 1 hour rest = 8 hours = 480 minutes
        assert_eq!(minutes, 480);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_calculate_day_hours_with_deduction() {
        let mut record = DayRecord::new(1, 1);
        record.start = Time::new(8, 0);
        record.end = Time::new(17, 0);
        record.deduction = Duration::new(1, 0);

        let settings = Settings {
            month: "202511".to_string(),
            rest: Duration::new(1, 0),
        };

        let mut warnings = Vec::new();
        let minutes = calculate_day_hours(&record, &settings, "Test", &mut warnings);

        // 9 hours - 1 hour rest - 1 hour deduction = 7 hours = 420 minutes
        assert_eq!(minutes, 420);
    }

    #[test]
    fn test_calculate_day_hours_short_day() {
        let mut record = DayRecord::new(1, 1);
        record.start = Time::new(8, 0);
        record.end = Time::new(10, 0);

        let settings = Settings {
            month: "202511".to_string(),
            rest: Duration::new(1, 0),
        };

        let mut warnings = Vec::new();
        let minutes = calculate_day_hours(&record, &settings, "Test", &mut warnings);

        // 2 hours, no rest applied = 120 minutes
        assert_eq!(minutes, 120);
    }

    #[test]
    fn test_calculate_day_hours_negative_result() {
        let mut record = DayRecord::new(1, 1);
        record.start = Time::new(8, 0);
        record.end = Time::new(10, 0);
        record.deduction = Duration::new(3, 0);

        let settings = Settings {
            month: "202511".to_string(),
            rest: Duration::new(1, 0),
        };

        let mut warnings = Vec::new();
        let minutes = calculate_day_hours(&record, &settings, "Test", &mut warnings);

        // Would be negative, clamped to 0
        assert_eq!(minutes, 0);
    }

    #[test]
    fn test_start_after_end() {
        let mut record = DayRecord::new(1, 1);
        record.start = Time::new(17, 0);
        record.end = Time::new(8, 0);

        let settings = Settings {
            month: "202511".to_string(),
            rest: Duration::new(1, 0),
        };

        let mut warnings = Vec::new();
        let minutes = calculate_day_hours(&record, &settings, "Test", &mut warnings);

        assert_eq!(minutes, 0);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_calculate_work_hours_single_person_single_day() {
        let mut timesheet = Timesheet {
            settings: Settings {
                month: "202511".to_string(),
                rest: Duration::new(1, 0),
            },
            persons: vec![],
        };

        let mut person = PersonRecord {
            name: "Tom".to_string(),
            days: vec![],
        };

        let mut day1 = DayRecord::new(1, 1);
        day1.start = Time::new(8, 0);
        day1.end = Time::new(17, 0);
        person.days.push(day1);

        timesheet.persons.push(person);

        let (results, warnings) = calculate_work_hours(&timesheet);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Tom");
        assert_eq!(results[0].work_days, 1);
        assert_eq!(results[0].work_hours, 8);
        assert_eq!(results[0].work_minutes, 0);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_calculate_work_hours_multiple_days() {
        let mut timesheet = Timesheet {
            settings: Settings {
                month: "202511".to_string(),
                rest: Duration::new(1, 0),
            },
            persons: vec![],
        };

        let mut person = PersonRecord {
            name: "Alice".to_string(),
            days: vec![],
        };

        // Day 1: 08:00-17:00 = 9h, rest 1h = 8h = 480 minutes
        let mut day1 = DayRecord::new(1, 1);
        day1.start = Time::new(8, 0);
        day1.end = Time::new(17, 0);
        person.days.push(day1);

        // Day 2: 08:15-17:30 = 9h15m, rest 1h = 8h15m = 495 minutes
        let mut day2 = DayRecord::new(2, 5);
        day2.start = Time::new(8, 15);
        day2.end = Time::new(17, 30);
        person.days.push(day2);

        timesheet.persons.push(person);

        let (results, warnings) = calculate_work_hours(&timesheet);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");
        assert_eq!(results[0].work_days, 2);
        // Total: 480 + 495 = 975 minutes = 16h15m
        assert_eq!(results[0].work_hours, 16);
        assert_eq!(results[0].work_minutes, 15);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_calculate_work_hours_with_deduction() {
        let mut timesheet = Timesheet {
            settings: Settings {
                month: "202511".to_string(),
                rest: Duration::new(1, 0),
            },
            persons: vec![],
        };

        let mut person = PersonRecord {
            name: "Bob".to_string(),
            days: vec![],
        };

        // 08:15-19:30 = 11h15m -> 675min, rest 1h = 615min, deduction 1h = 555min -> 9h15m
        let mut day1 = DayRecord::new(1, 1);
        day1.start = Time::new(8, 15);
        day1.end = Time::new(19, 30);
        day1.deduction = Duration::new(1, 0);
        person.days.push(day1);

        timesheet.persons.push(person);

        let (results, warnings) = calculate_work_hours(&timesheet);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Bob");
        assert_eq!(results[0].work_days, 1);
        assert_eq!(results[0].work_hours, 9);
        assert_eq!(results[0].work_minutes, 15);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_calculate_work_hours_multiple_people() {
        let mut timesheet = Timesheet {
            settings: Settings {
                month: "202511".to_string(),
                rest: Duration::new(1, 0),
            },
            persons: vec![],
        };

        // Person 1
        let mut person1 = PersonRecord {
            name: "Tom".to_string(),
            days: vec![],
        };
        let mut day1 = DayRecord::new(1, 1);
        day1.start = Time::new(8, 0);
        day1.end = Time::new(17, 0);
        person1.days.push(day1);
        timesheet.persons.push(person1);

        // Person 2
        let mut person2 = PersonRecord {
            name: "John".to_string(),
            days: vec![],
        };
        let mut day2 = DayRecord::new(1, 5);
        day2.start = Time::new(9, 0);
        day2.end = Time::new(18, 0);
        person2.days.push(day2);
        timesheet.persons.push(person2);

        let (results, warnings) = calculate_work_hours(&timesheet);

        assert_eq!(results.len(), 2);

        assert_eq!(results[0].name, "Tom");
        assert_eq!(results[0].work_hours, 8);
        assert_eq!(results[0].work_minutes, 0);

        assert_eq!(results[1].name, "John");
        assert_eq!(results[1].work_hours, 8);
        assert_eq!(results[1].work_minutes, 0);

        assert!(warnings.is_empty());
    }

    #[test]
    fn test_calculate_work_hours_invalid_date_excluded() {
        let mut timesheet = Timesheet {
            settings: Settings {
                month: "202511".to_string(),
                rest: Duration::new(1, 0),
            },
            persons: vec![],
        };

        let mut person = PersonRecord {
            name: "Charlie".to_string(),
            days: vec![],
        };

        // Valid day
        let mut day1 = DayRecord::new(1, 1);
        day1.start = Time::new(8, 0);
        day1.end = Time::new(17, 0);
        person.days.push(day1);

        // Invalid day (November has only 30 days)
        let mut day_invalid = DayRecord::new(31, 5);
        day_invalid.start = Time::new(8, 0);
        day_invalid.end = Time::new(17, 0);
        person.days.push(day_invalid);

        timesheet.persons.push(person);

        let (results, warnings) = calculate_work_hours(&timesheet);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Charlie");
        assert_eq!(results[0].work_days, 1); // Only valid date counts
        assert_eq!(results[0].work_hours, 8);
        assert_eq!(results[0].work_minutes, 0);
        assert_eq!(warnings.len(), 1); // One warning for invalid date
    }

    #[test]
    fn test_calculate_work_hours_exact_minutes() {
        let mut timesheet = Timesheet {
            settings: Settings {
                month: "202511".to_string(),
                rest: Duration::new(1, 0),
            },
            persons: vec![],
        };

        let mut person = PersonRecord {
            name: "Diana".to_string(),
            days: vec![],
        };

        // Day 1: 08:00-09:05 = 1h5m = 65 minutes (no rest, < 3 hours)
        let mut day1 = DayRecord::new(1, 1);
        day1.start = Time::new(8, 0);
        day1.end = Time::new(9, 5);
        person.days.push(day1);

        // Day 2: 08:00-09:23 = 1h23m = 83 minutes (no rest, < 3 hours)
        let mut day2 = DayRecord::new(2, 5);
        day2.start = Time::new(8, 0);
        day2.end = Time::new(9, 23);
        person.days.push(day2);

        timesheet.persons.push(person);

        let (results, warnings) = calculate_work_hours(&timesheet);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Diana");
        assert_eq!(results[0].work_days, 2);
        // Total: 65 + 83 = 148 minutes = 2h28m
        assert_eq!(results[0].work_hours, 2);
        assert_eq!(results[0].work_minutes, 28);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_calculate_work_hours_no_start_or_end() {
        let mut timesheet = Timesheet {
            settings: Settings {
                month: "202511".to_string(),
                rest: Duration::new(1, 0),
            },
            persons: vec![],
        };

        let mut person = PersonRecord {
            name: "Eve".to_string(),
            days: vec![],
        };

        // Day with no start time
        let mut day1 = DayRecord::new(1, 1);
        day1.end = Time::new(17, 0);
        person.days.push(day1);

        // Day with no end time
        let mut day2 = DayRecord::new(2, 5);
        day2.start = Time::new(8, 0);
        person.days.push(day2);

        // Valid day
        let mut day3 = DayRecord::new(3, 10);
        day3.start = Time::new(8, 0);
        day3.end = Time::new(17, 0);
        person.days.push(day3);

        timesheet.persons.push(person);

        let (results, warnings) = calculate_work_hours(&timesheet);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Eve");
        assert_eq!(results[0].work_days, 3); // All days count if date is valid
        assert_eq!(results[0].work_hours, 8); // Only day3 contributes
        assert_eq!(results[0].work_minutes, 0);
        assert!(warnings.is_empty());
    }
}
