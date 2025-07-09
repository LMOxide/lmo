/*!
 * Models Command Implementation
 * 
 * List and search available models.
 */

use anyhow::Result;
use crate::cli::ModelsCommand;
use crate::config::CliConfig;
use crate::output::{OutputFormatter, format_number, truncate_text};
use crate::utils::{create_client, check_server_health};

pub async fn handle(cmd: ModelsCommand, config: &CliConfig) -> Result<()> {
    let output = OutputFormatter::new(config, None, false);
    let client = create_client(config, None)?;
    
    // Check server health first
    check_server_health(&client, &output).await?;
    
    output.progress("Fetching models");
    
    // Fetch models with filters (local or remote)
    let (models_response, local_models_response) = if cmd.local {
        // Get local models - preserve both formats for enhanced display
        let local_response = client.list_local_models().await?;
        
        // Convert local models to ModelInfo format for filtering compatibility
        let models: Vec<lmoserver::shared_types::ModelInfo> = local_response.models.iter().map(|local_model| {
            lmoserver::shared_types::ModelInfo {
                id: local_model.filename.clone(),
                author: None, // Local models don't have author info
                downloads: 0, // Local models don't have download counts
                tags: vec![], // Could extract from filename/path later
                created_at: local_model.last_modified.to_rfc3339(),
                updated_at: local_model.last_modified.to_rfc3339(),
                pipeline_tag: None,
                library_name: None,
                files: vec![], // Local models don't have file info in this format
                supported_formats: vec![], // Could be populated later
            }
        }).collect();
        
        let response = lmoclient::models::ModelListResponse {
            models,
            total: Some(local_response.total_count as u32),
            has_more: false,
        };
        
        (response, Some(local_response))
    } else {
        // Get remote models from HuggingFace
        (client.list_models().await?, None)
    };
    
    output.progress_done();
    
    if models_response.models.is_empty() {
        output.warning("No models found matching the criteria");
        return Ok(());
    }
    
    // Apply client-side filtering and sorting
    let mut models = models_response.models;
    
    // Filter by search term
    if let Some(ref search) = cmd.search {
        models.retain(|m| m.id.to_lowercase().contains(&search.to_lowercase()));
    }
    
    // Filter by author
    if let Some(ref author) = cmd.author {
        models.retain(|m| {
            m.author.as_ref()
                .map(|a| a.to_lowercase().contains(&author.to_lowercase()))
                .unwrap_or(false)
        });
    }
    
    // Filter by tags
    if let Some(ref tags) = cmd.tags {
        let search_tags: Vec<&str> = tags.split(',').map(|t| t.trim()).collect();
        models.retain(|m| {
            search_tags.iter().any(|tag| {
                m.tags.iter().any(|t| t.to_lowercase().contains(&tag.to_lowercase()))
            })
        });
    }
    
    // Filter by pipeline
    if let Some(ref pipeline) = cmd.pipeline {
        models.retain(|m| {
            m.pipeline_tag.as_ref()
                .map(|p| p.to_lowercase().contains(&pipeline.to_lowercase()))
                .unwrap_or(false)
        });
    }
    
    // Sort models
    match cmd.sort.as_str() {
        "downloads" => {
            if cmd.direction == "asc" {
                models.sort_by(|a, b| a.downloads.cmp(&b.downloads));
            } else {
                models.sort_by(|a, b| b.downloads.cmp(&a.downloads));
            }
        }
        "author" => {
            if cmd.direction == "asc" {
                models.sort_by(|a, b| a.author.cmp(&b.author));
            } else {
                models.sort_by(|a, b| b.author.cmp(&a.author));
            }
        }
        "created" => {
            if cmd.direction == "asc" {
                models.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            } else {
                models.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
        }
        _ => {} // Keep original order
    }
    
    // Limit results
    models.truncate(cmd.limit as usize);
    
    // Display results
    let title = if cmd.local {
        format!("Local Models ({} found)", models.len())
    } else {
        format!("Available Models ({} found)", models.len())
    };
    output.header(&title);
    println!();
    
    match &config.output_format[..] {
        "json" => {
            output.print(&models)?;
        }
        "yaml" => {
            output.print(&models)?;
        }
        _ => {
            // Table format - adjust headers based on local vs remote
            if cmd.local {
                // Enhanced local models display with compatibility information
                println!("{:<40} {:<10} {:<12} {:<12} {:<3} {:<30} {:<12}", 
                    "Model ID", "Format", "Backend", "Size", "✓", "Compatibility", "Status");
                println!("{}", "-".repeat(119));
                
                if let Some(ref local_response) = local_models_response {
                    for local_model in &local_response.models {
                        let (format, backend, compat_icon, compat_text) = extract_model_info(local_model);
                        let size = format_bytes(local_model.size_bytes);
                        let status = if local_model.is_loaded { "Loaded" } else { "Available" };
                        
                        println!("{:<40} {:<10} {:<12} {:<12} {:<3} {:<30} {:<12}", 
                            truncate_text(&local_model.filename, 40),
                            format,
                            backend,
                            size,
                            compat_icon,
                            truncate_text(&compat_text, 30),
                            status
                        );
                    }
                } else {
                    // Fallback to basic display if local_models_response is not available
                    for model in &models {
                        println!("{:<40} {:<10} {:<12} {:<12} {:<3} {:<30} {:<12}", 
                            truncate_text(&model.id, 40),
                            "Unknown",
                            "Auto",
                            "N/A",
                            "❓",
                            "Unknown",
                            "Available"
                        );
                    }
                }
            } else {
                // Remote models display
                println!("{:<40} {:<20} {:<15} {:<20} {:<30}", 
                    "Model ID", "Author", "Downloads", "Pipeline", "Tags");
                println!("{}", "-".repeat(125));
                
                for model in &models {
                    let author = model.author.as_deref().unwrap_or("Unknown");
                    let pipeline = model.pipeline_tag.as_deref().unwrap_or("Unknown");
                    let tags = if model.tags.is_empty() {
                        "None".to_string()
                    } else {
                        truncate_text(&model.tags.join(", "), 30)
                    };
                    
                    println!("{:<40} {:<20} {:<15} {:<20} {:<30}", 
                        truncate_text(&model.id, 40),
                        truncate_text(author, 20),
                        format_number(model.downloads),
                        truncate_text(pipeline, 20),
                        tags
                    );
                }
            }
        }
    }
    
    println!();
    output.info(&format!("Showing {} of {} total models", models.len(), models_response.total.unwrap_or(models.len() as u32)));
    
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

/// Extract model format, backend, and compatibility info from server metadata
fn extract_model_info(local_model: &lmoclient::models::LocalModelInfo) -> (String, String, String, String) {
    if let Some(metadata) = &local_model.metadata {
        // Extract format from server metadata
        let format = metadata.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        
        // Extract backend from server metadata
        let backend = metadata.get("backend")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        
        // Extract compatibility info from server metadata
        let compat_icon = metadata.get("compatibility_icon")
            .and_then(|v| v.as_str())
            .unwrap_or("❓")
            .to_string();
        
        // Enhanced compatibility text with confidence and detection info
        let compat_text = if let Some(format_detection) = metadata.get("format_detection") {
            let confidence = format_detection.get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            
            let detection_method = format_detection.get("detection_method")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            
            let base_text = metadata.get("compatibility_text")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            
            // Format: "Compatible (95% config_metadata)" or "Universal (90% structure_analysis)"
            format!("{} ({:.0}% {})", base_text, confidence * 100.0, detection_method)
        } else {
            metadata.get("compatibility_text")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string()
        };
        
        (format, backend, compat_icon, compat_text)
    } else {
        // No metadata available from server
        ("Unknown".to_string(), "Unknown".to_string(), "❓".to_string(), "No metadata".to_string())
    }
}