/*!
 * Download Command Implementation
 * 
 * Download models from remote repositories.
 */

use anyhow::Result;
use crate::cli::DownloadCommand;
use crate::config::CliConfig;
use crate::output::OutputFormatter;
use crate::utils::{create_client, check_server_health};

pub async fn handle(cmd: DownloadCommand, config: &CliConfig) -> Result<()> {
    let output = OutputFormatter::new(config, None, false);
    let client = create_client(config, None)?;
    
    // Check server health first
    check_server_health(&client, &output).await?;
    
    output.header(&format!("Downloading Model: {}", cmd.model_name));
    println!();
    
    // Validate model name format
    if !cmd.model_name.contains('/') {
        output.warning("Model name should include organization/repository (e.g., 'microsoft/DialoGPT-small')");
        output.info("Attempting to download anyway...");
    }
    
    // Show download configuration
    output.subheader("Download Configuration");
    output.key_value("Model Name", &cmd.model_name);
    
    if let Some(ref format) = cmd.format {
        output.key_value("Format Hint", format);
    }
    
    if cmd.force {
        output.key_value("Force Re-download", "Yes");
    }
    
    if let Some(ref directory) = cmd.directory {
        output.key_value("Custom Directory", directory);
    }
    
    println!();
    
    // Prepare download request
    output.progress("Preparing download request");
    
    let download_request = lmoclient::models::DownloadModelRequest {
        model_name: cmd.model_name.clone(),
        format_hint: cmd.format.clone(),
        force_redownload: cmd.force,
        custom_directory: cmd.directory.clone(),
    };
    
    output.progress_done();
    
    // Send download request
    output.progress("Sending download request to server");
    let result = client.download_model(download_request).await;
    output.progress_done();
    
    match result {
        Ok(response) => {
            if response.success {
                output.success(&format!("✓ Model download completed: {}", response.model_name));
                
                if let Some(ref model_id) = response.model_id {
                    output.key_value("Model ID", model_id);
                }
                
                if let Some(ref path) = response.download_path {
                    output.key_value("Download Path", path);
                }
                
                if let Some(ref format) = response.detected_format {
                    output.key_value("Detected Format", format);
                }
                
                if let Some(size) = response.size_bytes {
                    output.key_value("Size", &format!("{} bytes", size));
                }
                
                if let Some(duration) = response.duration_ms {
                    output.key_value("Download Time", &format!("{}ms", duration));
                }
                
                if let Some(ref metadata) = response.metadata {
                    if let Some(files) = metadata.get("downloaded_files") {
                        println!();
                        output.info("Downloaded files:");
                        if let serde_json::Value::Array(files_array) = files {
                            for file in files_array {
                                if let serde_json::Value::String(file_str) = file {
                                    output.info(&format!("  • {}", file_str));
                                }
                            }
                        }
                    }
                    
                    if let Some(registry_status) = metadata.get("registry_status") {
                        println!();
                        output.info(&format!("Registry Status: {}", registry_status));
                    }
                }
                
                println!();
                output.success("Model is now available for loading with 'lmo load'");
            } else {
                output.warning(&format!("Model download failed: {}", response.message));
                
                // Show what was attempted
                println!();
                output.subheader("Attempted Download Operation");
                output.key_value("Model Name", &cmd.model_name);
                
                if let Some(ref format) = cmd.format {
                    output.key_value("Format Hint", format);
                }
                
                if cmd.force {
                    output.key_value("Force Re-download", "Yes");
                }
                
                if let Some(ref error_details) = response.error_details {
                    println!();
                    output.info("Error details:");
                    output.info(&format!("  {}", error_details));
                }
            }
        },
        Err(e) => {
            output.warning(&format!("Failed to communicate with server: {}", e));
            
            // Provide helpful suggestions
            println!();
            output.info("Troubleshooting suggestions:");
            output.info("  • Ensure the server is running: lmo health");
            output.info("  • Check model name format: organization/model-name");
            output.info("  • Verify network connectivity");
            output.info("  • Check server logs for detailed error information");
        }
    }
    
    Ok(())
}