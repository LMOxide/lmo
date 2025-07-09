/*!
 * Download Command Implementation
 * 
 * Download models from remote repositories.
 */

use anyhow::Result;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::pin::Pin;
use tokio::signal;

use crate::cli::DownloadCommand;
use crate::config::CliConfig;
use crate::output::OutputFormatter;
use crate::utils::{create_client, check_server_health};

/// Handle download command with real-time progress
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
    output.progress("Starting download...");
    
    let download_request = lmoclient::models::DownloadModelRequest {
        model_name: cmd.model_name.clone(),
        format_hint: cmd.format.clone(),
        force_redownload: cmd.force,
        custom_directory: cmd.directory.clone(),
    };
    
    // Start the download and get download ID
    let start_response = client.download_start(download_request).await?;
    output.progress_done();
    
    output.success(&format!("✓ Download started: {}", start_response.download_id));
    if let Some(size) = start_response.estimated_size_bytes {
        output.key_value("Estimated Size", &format_bytes(size));
    }
    println!();
    
    // Create progress bar
    let progress_bar = ProgressBar::new(100);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {percent:>3}% {msg}")
            .expect("Invalid progress bar template")
            .progress_chars("#>-")
    );
    
    // Start SSE stream for progress updates
    let progress_stream = client.download_progress_stream(&start_response.download_id).await?;
    let mut stream = Box::pin(progress_stream.into_stream());
    
    // Handle Ctrl+C for download cancellation
    let download_id = start_response.download_id.clone();
    let client_clone = client.clone();
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                eprintln!("\nReceived Ctrl+C, cancelling download...");
                if let Err(e) = client_clone.download_cancel(&download_id).await {
                    eprintln!("Error cancelling download: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error setting up Ctrl+C handler: {}", e);
            }
        }
    });
    
    // Stream progress updates with timeout
    let mut last_status = String::new();
    let mut no_events_count = 0;
    
    loop {
        // Add timeout to prevent hanging
        let timeout_duration = tokio::time::Duration::from_secs(30);
        
        match tokio::time::timeout(timeout_duration, stream.next()).await {
            Ok(Some(event_result)) => {
                no_events_count = 0; // Reset counter
                
                match event_result {
                    Ok(event) => {
                        let progress = &event.state.progress;
                        
                        // Update progress bar (round to nearest integer)
                        progress_bar.set_position(progress.percentage.round() as u64);
                        
                        // Create progress message
                        let mut msg_parts = Vec::new();
                        
                        if progress.total_bytes > 0 {
                            msg_parts.push(format!(
                                "{}/{}",
                                format_bytes(progress.downloaded_bytes),
                                format_bytes(progress.total_bytes)
                            ));
                        }
                        
                        if progress.speed_bps > 0.0 {
                            msg_parts.push(format!("{}/s", format_bytes(progress.speed_bps as u64)));
                        }
                        
                        if let Some(eta) = progress.eta_seconds {
                            if eta > 0.0 {
                                msg_parts.push(format!("ETA: {}s", eta as u64));
                            }
                        }
                        
                        if let Some(ref current_file) = progress.current_file {
                            msg_parts.push(format!("File: {}", current_file));
                        }
                        
                        msg_parts.push(format!("Files: {}/{}", progress.files_completed, progress.total_files));
                        
                        let status_msg = msg_parts.join(" | ");
                        progress_bar.set_message(status_msg.clone());
                        
                        // Update status only if changed
                        if last_status != format!("{:?}", event.state.status) {
                            last_status = format!("{:?}", event.state.status);
                            match event.event_type {
                                lmoclient::DownloadEventType::Started => {
                                    progress_bar.println("📥 Download started");
                                }
                                lmoclient::DownloadEventType::FileCompleted => {
                                    if let Some(ref file) = progress.current_file {
                                        progress_bar.println(&format!("✓ Completed: {}", file));
                                    }
                                }
                                lmoclient::DownloadEventType::Paused => {
                                    progress_bar.println("⏸️  Download paused");
                                }
                                lmoclient::DownloadEventType::Resumed => {
                                    progress_bar.println("▶️  Download resumed");
                                }
                                lmoclient::DownloadEventType::Completed => {
                                    progress_bar.finish_with_message("✅ Download completed!");
                                    break;
                                }
                                lmoclient::DownloadEventType::Failed => {
                                    progress_bar.abandon_with_message("❌ Download failed!");
                                    if let Some(ref error) = event.state.error_message {
                                        output.warning(&format!("Error: {}", error));
                                    }
                                    return Ok(());
                                }
                                lmoclient::DownloadEventType::Cancelled => {
                                    progress_bar.abandon_with_message("🛑 Download cancelled");
                                    return Ok(());
                                }
                                _ => {} // Progress updates don't need special handling
                            }
                        }
                    }
                    Err(e) => {
                        progress_bar.abandon_with_message("❌ Stream error!");
                        output.warning(&format!("Stream error: {}", e));
                        
                        // Check if this is a common error and provide helpful guidance
                        let error_msg = e.to_string();
                        if error_msg.contains("connection closed") || error_msg.contains("stream ended") {
                            output.info("Download may have completed or failed. Check server logs for details.");
                        } else if error_msg.contains("decoding response body") {
                            output.info("Network connection issue. The download may continue in the background.");
                        } else {
                            output.info("Try running the download again or check the server status.");
                        }
                        break;
                    }
                }
            }
            Ok(None) => {
                // Stream ended
                progress_bar.abandon_with_message("📡 Stream ended");
                output.info("Download stream ended");
                break;
            }
            Err(_timeout) => {
                // Timeout occurred
                no_events_count += 1;
                if no_events_count >= 3 {
                    progress_bar.abandon_with_message("⏰ Stream timeout");
                    output.warning("Download stream timed out - no progress updates received");
                    break;
                } else {
                    progress_bar.set_message(format!("Waiting for updates... ({})", no_events_count));
                }
            }
        }
    }
    
    println!();
    output.success("Model is now available for loading with 'lmo load'");
    
    Ok(())
}

/// Format bytes into human readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}