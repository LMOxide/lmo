/*!
 * Unload Command Implementation
 * 
 * Unload loaded models.
 */

use anyhow::Result;
use crate::cli::UnloadCommand;
use crate::config::CliConfig;
use crate::output::OutputFormatter;
use crate::utils::{create_client, check_server_health};

pub async fn handle(cmd: UnloadCommand, config: &CliConfig) -> Result<()> {
    let output = OutputFormatter::new(config, None, false);
    let client = create_client(config, None)?;
    
    // Check server health first
    check_server_health(&client, &output).await?;
    
    output.header(&format!("Unloading Model Instance: {}", cmd.instance_id));
    println!();
    
    // Send unload request
    output.progress("Sending unload request to server");
    
    let unload_request = lmoclient::models::UnloadModelRequest {
        instance_id: cmd.instance_id.clone(),
    };
    
    let result = client.unload_model(unload_request).await;
    output.progress_done();
    
    match result {
        Ok(response) => {
            if response.success {
                output.success(&format!("✓ Model unloaded: {}", response.model_id));
                output.key_value("Instance ID", &response.instance_id);
                output.key_value("Memory Freed", &format!("{}MB", response.memory_freed_bytes / 1024 / 1024));
                output.key_value("Duration", &format!("{}ms", response.duration_ms));
            } else {
                output.warning(&format!("Model unload failed: {}", response.message));
                output.key_value("Instance ID", &cmd.instance_id);
                
                if cmd.force {
                    output.info("Force unload was requested but failed");
                }
            }
        },
        Err(e) => {
            output.warning(&format!("Failed to communicate with server: {}", e));
            
            println!();
            output.subheader("Attempted Unload Operation");
            output.key_value("Instance ID", &cmd.instance_id);
            
            if cmd.force {
                output.key_value("Force Unload", "Yes");
            }
        }
    }
    
    Ok(())
}