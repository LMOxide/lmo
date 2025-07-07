/*!
 * Status Command Implementation
 * 
 * Show status of loaded models and server information.
 */

use anyhow::Result;
use crate::cli::StatusCommand;
use crate::config::CliConfig;
use crate::output::{OutputFormatter, format_number};
use crate::utils::{create_client, check_server_health, format_duration};

pub async fn handle(cmd: StatusCommand, config: &CliConfig) -> Result<()> {
    let output = OutputFormatter::new(config, None, false);
    let client = create_client(config, None)?;
    
    // Check server health first
    check_server_health(&client, &output).await?;
    
    if cmd.detailed {
        output.header("Server Status");
        println!();
        
        // Get server health information
        output.progress("Getting server status");
        let health = client.health().await?;
        output.progress_done();
        
        output.key_value("Server Status", &health.status);
        output.key_value("Server Version", &health.server_version);
        output.key_value("Uptime", &format_duration(health.uptime_seconds));
        output.key_value("Server URL", &client.config().server_url);
        
        println!();
        
        // Get available models count
        output.progress("Getting model information");
        let models_response = client.list_models().await?;
        output.progress_done();
        
        output.key_value("Available Models", &format_number(models_response.models.len() as u64));
        
        if let Some(total) = models_response.total {
            output.key_value("Total in Registry", &format_number(total as u64));
        }
        
        println!();
        
        // Model management status
        output.subheader("Model Management Status");
        output.key_value("Load/Unload Support", "Pending Universal Model Engine integration");
        output.key_value("Current Capability", "Model discovery and health monitoring");
        
        println!();
        output.info("ℹ Model loading features will be available once the server's Universal Model Engine system is fully integrated.");
        
    } else {
        // Simple status overview
        output.progress("Checking status");
        
        let health = client.health().await?;
        let models_response = client.list_models().await?;
        
        output.progress_done();
        
        let status_icon = match health.status.as_str() {
            "healthy" => "✓",
            "degraded" => "⚠",
            "unhealthy" => "✗",
            _ => "?",
        };
        
        output.success(&format!(
            "{} Server is {} • {} models available • Uptime: {}",
            status_icon,
            health.status,
            models_response.models.len(),
            format_duration(health.uptime_seconds)
        ));
        
        if cmd.refresh.is_some() {
            output.info("Note: Watch mode not yet implemented. Use health command for monitoring.");
        }
    }
    
    // Handle specific model status
    if let Some(model_id) = cmd.model {
        println!();
        output.warning(&format!(
            "Model-specific status for '{}' not yet available. Model loading functionality is pending server integration.",
            model_id
        ));
    }
    
    Ok(())
}