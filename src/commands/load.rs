/*!
 * Load Command Implementation
 * 
 * Load models for inference.
 */

use anyhow::Result;
use crate::cli::LoadCommand;
use crate::config::CliConfig;
use crate::output::OutputFormatter;
use crate::utils::{create_client, check_server_health};

pub async fn handle(cmd: LoadCommand, config: &CliConfig) -> Result<()> {
    let output = OutputFormatter::new(config, None, false);
    let client = create_client(config, None)?;
    
    // Check server health first
    check_server_health(&client, &output).await?;
    
    output.header(&format!("Loading Model: {}", cmd.model_id));
    println!();
    
    // Verify model exists in registry
    output.progress("Verifying model exists");
    let models_response = client.list_models().await?;
    let model_found = models_response.models.iter()
        .any(|m| m.id == cmd.model_id || m.id.contains(&cmd.model_id));
    
    output.progress_done();
    
    if !model_found {
        output.warning(&format!("Model '{}' not found in available models registry.", cmd.model_id));
        output.info("Use 'lmo models --search <term>' to find available models.");
        return Ok(());
    }
    
    output.success(&format!("✓ Model '{}' found in registry", cmd.model_id));
    
    // Attempt to load the model
    println!();
    output.progress("Sending load request to server");
    
    let load_request = lmoclient::models::LoadModelRequest {
        model_id: cmd.model_id.clone(),
        filename: cmd.filename.clone(),
        config: Some(lmoclient::models::LoadModelConfig {
            max_memory_gb: None,
            gpu_layers: None,
            context_size: None,
            force_reload: cmd.force,
        }),
    };
    
    let result = client.load_model(load_request).await;
    output.progress_done();
    
    match result {
        Ok(response) => {
            if response.success {
                output.success(&format!("✓ Model load initiated: {}", response.model_id));
                
                if let Some(instance_id) = response.instance_id {
                    output.key_value("Instance ID", &instance_id);
                }
                
                if let Some(duration) = response.duration_ms {
                    output.key_value("Response Time", &format!("{}ms", duration));
                }
                
                if let Some(ref metadata) = response.metadata {
                    if let Some(status) = metadata.get("integration_status") {
                        println!();
                        output.info(&format!("Status: {}", status));
                    }
                    
                    if let Some(features) = metadata.get("expected_features") {
                        output.info("Expected features when fully integrated:");
                        if let serde_json::Value::Array(features_array) = features {
                            for feature in features_array {
                                if let serde_json::Value::String(feature_str) = feature {
                                    output.info(&format!("  • {}", feature_str));
                                }
                            }
                        }
                    }
                }
            } else {
                output.warning(&format!("Model load request failed: {}", response.message));
                
                // Show what was attempted
                println!();
                output.subheader("Attempted Load Operation");
                output.key_value("Model ID", &cmd.model_id);
                
                if let Some(ref filename) = cmd.filename {
                    output.key_value("Specific File", filename);
                }
                
                if cmd.force {
                    output.key_value("Force Reload", "Yes");
                }
            }
        },
        Err(e) => {
            output.warning(&format!("Failed to communicate with server: {}", e));
        }
    }
    
    Ok(())
}