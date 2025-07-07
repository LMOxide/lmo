/*!
 * Health Command Implementation
 * 
 * Check server health and status.
 */

use anyhow::Result;
use crate::cli::HealthCommand;
use crate::config::CliConfig;
use crate::output::{OutputFormatter, format_bytes};
use crate::utils::{create_client, format_duration};

pub async fn handle(cmd: HealthCommand, config: &CliConfig) -> Result<()> {
    let output = OutputFormatter::new(config, None, false);
    let client = create_client(config, None)?;
    
    output.progress("Checking server health");
    
    let health = client.health().await?;
    
    output.progress_done();
    
    if cmd.detailed {
        // Detailed health information
        output.header("Server Health Status");
        println!();
        
        output.key_value("Status", &health.status);
        
        output.key_value("Version", &health.server_version);
        
        output.key_value("Uptime", &format_duration(health.uptime_seconds));
        
        output.key_value("Timestamp", &health.timestamp);
        
        println!();
    } else {
        // Simple health check
        if health.status == "healthy" {
            output.success("Server is healthy");
        } else {
            output.warning(&format!("Server status: {}", health.status));
        }
        
        output.info(&format!("Server version: {}", health.server_version));
    }
    
    Ok(())
}