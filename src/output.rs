use crate::calculator::PersonResult;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Generate output filename based on input filename
pub fn generate_output_filename(input_filename: &str, total_files: usize) -> String {
    if total_files == 1 {
        "chronosheet_result.toml".to_string()
    } else {
        // Extract base name without extension
        let base_name = Path::new(input_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        format!("chronosheet_result_{}.toml", base_name)
    }
}

/// Write results to TOML file
pub fn write_results(
    filename: &str,
    month: &str,
    results: &[PersonResult],
) -> std::io::Result<()> {
    let mut file = fs::File::create(filename)?;

    // Write month
    writeln!(file, "month = \"{}\"", month)?;

    // Write each person as TOML array table
    for person in results {
        writeln!(
            file,
            "\n[[sheets]]\nname = \"{}\"\nwork_days = {}\nwork_times = {}:{:02}",
            person.name, person.work_days, person.work_hours, person.work_minutes
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculator::PersonResult;

    #[test]
    fn test_generate_output_filename_single() {
        assert_eq!(
            generate_output_filename("time.txt", 1),
            "chronosheet_result.toml"
        );
    }

    #[test]
    fn test_generate_output_filename_multiple() {
        assert_eq!(
            generate_output_filename("time.txt", 2),
            "chronosheet_result_time.toml"
        );
        assert_eq!(
            generate_output_filename("november.txt", 3),
            "chronosheet_result_november.toml"
        );
    }

    #[test]
    fn test_write_results() {
        use std::fs;

        let results = vec![
            PersonResult {
                name: "Tom".to_string(),
                work_days: 2,
                work_hours: 17,
                work_minutes: 30,
            },
            PersonResult {
                name: "John".to_string(),
                work_days: 1,
                work_hours: 10,
                work_minutes: 15,
            },
        ];

        let filename = "test_output.toml";
        write_results(filename, "202511", &results).unwrap();

        let content = fs::read_to_string(filename).unwrap();
        assert!(content.contains("month = \"202511\""));
        assert!(content.contains("[[sheets]]"));
        assert!(content.contains("name = \"Tom\""));
        assert!(content.contains("work_days = 2"));
        assert!(content.contains("work_times = 17:30"));

        fs::remove_file(filename).ok();
    }
}
