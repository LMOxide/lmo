/*!
 * Output Formatting
 * 
 * Handles different output formats (table, JSON, YAML) and styling.
 */

use std::io::{self, Write};
use anyhow::Result;
use colored::*;
use serde::Serialize;

use crate::config::CliConfig;

pub struct OutputFormatter {
    format: OutputFormat,
    enable_colors: bool,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

impl OutputFormatter {
    pub fn new(config: &CliConfig, format_override: Option<&str>, no_color: bool) -> Self {
        let format = match format_override.unwrap_or(&config.output_format).to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "yaml" | "yml" => OutputFormat::Yaml,
            _ => OutputFormat::Table,
        };

        Self {
            format,
            enable_colors: config.enable_colors && !no_color,
        }
    }

    /// Format and print data
    pub fn print<T: Serialize>(&self, data: &T) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(data)?;
                println!("{}", json);
            }
            OutputFormat::Yaml => {
                let yaml = serde_yaml::to_string(data)?;
                println!("{}", yaml);
            }
            OutputFormat::Table => {
                // For table format, we need custom implementations per data type
                // This is a fallback to JSON
                let json = serde_json::to_string_pretty(data)?;
                println!("{}", json);
            }
        }
        Ok(())
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        if self.enable_colors {
            println!("{} {}", "✓".green().bold(), message);
        } else {
            println!("✓ {}", message);
        }
    }

    /// Print an error message
    pub fn error(&self, message: &str) {
        if self.enable_colors {
            eprintln!("{} {}", "✗".red().bold(), message.red());
        } else {
            eprintln!("✗ {}", message);
        }
    }

    /// Print a warning message
    pub fn warning(&self, message: &str) {
        if self.enable_colors {
            println!("{} {}", "⚠".yellow().bold(), message.yellow());
        } else {
            println!("⚠ {}", message);
        }
    }

    /// Print an info message
    pub fn info(&self, message: &str) {
        if self.enable_colors {
            println!("{} {}", "ℹ".blue().bold(), message);
        } else {
            println!("ℹ {}", message);
        }
    }

    /// Print a header
    pub fn header(&self, text: &str) {
        if self.enable_colors {
            println!("{}", text.bold().underline());
        } else {
            println!("{}", text);
            println!("{}", "=".repeat(text.len()));
        }
    }

    /// Print a subheader
    pub fn subheader(&self, text: &str) {
        if self.enable_colors {
            println!("{}", text.bold());
        } else {
            println!("{}", text);
            println!("{}", "-".repeat(text.len()));
        }
    }

    /// Format a table row
    pub fn table_row(&self, cells: &[&str]) -> String {
        cells.join(" | ")
    }

    /// Format a key-value pair
    pub fn key_value(&self, key: &str, value: &str) {
        if self.enable_colors {
            println!("{}: {}", key.cyan().bold(), value);
        } else {
            println!("{}: {}", key, value);
        }
    }

    /// Print a progress indicator
    pub fn progress(&self, message: &str) {
        if self.enable_colors {
            print!("{} {}... ", "○".blue(), message);
        } else {
            print!("○ {}... ", message);
        }
        io::stdout().flush().unwrap();
    }

    /// Print completion of progress
    pub fn progress_done(&self) {
        if self.enable_colors {
            println!("{}", "done".green());
        } else {
            println!("done");
        }
    }

    /// Print failure of progress
    pub fn progress_failed(&self, error: &str) {
        if self.enable_colors {
            println!("{}: {}", "failed".red(), error.red());
        } else {
            println!("failed: {}", error);
        }
    }
    
    /// Print a status message
    pub fn status(&self, message: &str) {
        if self.enable_colors {
            print!("{} {}... ", "⚡".yellow(), message);
        } else {
            print!("⚡ {}... ", message);
        }
        io::stdout().flush().unwrap();
    }
    
    /// Print a debug message
    pub fn debug(&self, message: &str) {
        if self.enable_colors {
            println!("{} {}", "🔍".dimmed(), message.dimmed());
        } else {
            println!("🔍 {}", message);
        }
    }
}

/// Helper to format file sizes
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Helper to format numbers with commas
pub fn format_number(num: u64) -> String {
    let num_str = num.to_string();
    let mut result = String::new();
    
    for (i, c) in num_str.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    
    result.chars().rev().collect()
}

/// Helper to truncate text with ellipsis
pub fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}