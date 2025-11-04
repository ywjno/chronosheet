mod types;
mod error;
mod parser;
mod calculator;
mod output;

use error::ChronoError;
use std::env;
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), ChronoError> {
    // Get current directory
    let current_dir = env::current_dir()
        .map_err(|e| ChronoError::IoError(e))?;

    // Find all .txt files
    let txt_files = parser::find_txt_files(&current_dir)?;

    if txt_files.is_empty() {
        eprintln!("No .txt files found in current directory");
        return Ok(());
    }

    let total_files = txt_files.len();

    // Process each file
    for filename in txt_files {
        println!("Processing {}...", filename);

        // Parse the file
        let mut parser = parser::Parser::new(&filename)?;
        let timesheet = parser.parse()?;

        // Print parser warnings
        for warning in parser.warnings() {
            warning.print();
        }

        // Calculate work hours
        let (results, calc_warnings) = calculator::calculate_work_hours(&timesheet);

        // Print calculation warnings
        for warning in calc_warnings {
            warning.print();
        }

        // Generate output filename
        let output_filename = output::generate_output_filename(&filename, total_files);

        // Write results
        output::write_results(
            &output_filename,
            &timesheet.settings.month,
            &results,
        )?;

        println!("Results written to {}", output_filename);
    }

    Ok(())
}
