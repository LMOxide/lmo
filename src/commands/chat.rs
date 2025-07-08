/*!
 * Chat Command Implementation
 * 
 * Interactive chat with loaded models.
 */

use anyhow::{Context, Result};
use lmoclient::{LmoClient, models::LoadModelRequest};
use lmoserver::shared_types::{ChatCompletionRequest, ChatMessage};
use std::io::{self, Write};

use crate::cli::ChatCommand;
use crate::config::CliConfig;
use crate::output::OutputFormatter;

pub async fn handle(cmd: ChatCommand, config: &CliConfig, verbose: bool) -> Result<()> {
    let output = OutputFormatter::new(config, None, false);
    
    // Create client
    let client = LmoClient::with_url(&config.server_url)
        .context("Failed to create LMO client")?;
    
    // Check server health
    output.status("Checking server health...");
    match client.health().await {
        Ok(_) => output.success("Server is healthy"),
        Err(e) => {
            output.error(&format!("Server health check failed: {}", e));
            return Ok(());
        }
    }
    
    // Determine model to use
    let model_name = if let Some(ref model) = cmd.model {
        model.clone()
    } else {
        // List loaded models and prompt user to select
        let loaded_models = client.loaded_models().await
            .context("Failed to get loaded models")?;
        
        if loaded_models.is_empty() {
            output.warning("No models are currently loaded. Use 'lmo load <model>' first.");
            return Ok(());
        }
        
        if loaded_models.len() == 1 {
            loaded_models[0].model_id.clone()
        } else {
            output.info("Multiple models loaded. Please specify which model to use:");
            for (i, model) in loaded_models.iter().enumerate() {
                println!("  {}: {} ({})", i + 1, model.model_id, model.status);
            }
            output.warning("Use --model flag to specify a model");
            return Ok(());
        }
    };
    
    // Ensure model is loaded
    if let Some(ref model) = cmd.model {
        output.status(&format!("Ensuring model {} is loaded...", model));
        let load_request = LoadModelRequest {
            model_id: model.clone(),
            filename: None,
            config: None,
        };
        
        match client.load_model(load_request).await {
            Ok(response) => {
                if response.success {
                    output.success(&format!("Model {} is ready", model));
                } else {
                    output.error(&format!("Failed to load model: {}", response.message));
                    return Ok(());
                }
            }
            Err(e) => {
                output.error(&format!("Error loading model: {}", e));
                return Ok(());
            }
        }
    }
    
    // Single message mode
    if let Some(input_message) = cmd.input {
        let mut messages = vec![];
        
        // Add system prompt if provided
        if let Some(system) = cmd.system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system,
                name: None,
            });
        }
        
        // Add user message
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: input_message,
            name: None,
        });
        
        let request = ChatCompletionRequest {
            model: model_name,
            messages,
            temperature: Some(cmd.temperature),
            max_tokens: Some(cmd.max_tokens),
            stream: Some(cmd.stream),
            top_p: None,
            n: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            seed: None,
            user: None,
        };
        
        output.status("Generating response...");
        match client.chat_completion(request).await {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    output.info("Response:");
                    println!("{}", choice.message.content);
                    
                    // Show usage statistics if available
                    if let Some(usage) = response.usage {
                        output.debug(&format!(
                            "Tokens: {} prompt + {} completion = {} total",
                            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                        ));
                    }
                } else {
                    output.warning("No response generated");
                }
            }
            Err(e) => {
                output.error(&format!("Chat completion failed: {}", e));
            }
        }
        
        return Ok(());
    }
    
    // Interactive mode
    output.info(&format!("Starting interactive chat with model: {}", model_name));
    output.info("Type 'exit' or 'quit' to end the conversation");
    output.info("Type '/help' for available commands");
    println!();
    
    let mut conversation_history = vec![];
    
    // Add system prompt if provided
    if let Some(system) = cmd.system {
        conversation_history.push(ChatMessage {
            role: "system".to_string(),
            content: system,
            name: None,
        });
        output.debug("System prompt added to conversation");
    }
    
    loop {
        // Get user input
        print!("You: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {},
            Err(e) => {
                output.error(&format!("Failed to read input: {}", e));
                break;
            }
        }
        
        let input = input.trim();
        
        // Handle special commands
        if input.is_empty() {
            continue;
        }
        
        if input == "exit" || input == "quit" {
            output.info("Goodbye!");
            break;
        }
        
        if input == "/help" {
            println!("Available commands:");
            println!("  exit, quit  - End the conversation");
            println!("  /help       - Show this help");
            println!("  /clear      - Clear conversation history");
            println!("  /history    - Show conversation history");
            continue;
        }
        
        if input == "/clear" {
            // Keep system message if present
            let system_msg = conversation_history.iter()
                .find(|msg| msg.role == "system")
                .cloned();
            conversation_history.clear();
            if let Some(system) = system_msg {
                conversation_history.push(system);
            }
            output.info("Conversation history cleared");
            continue;
        }
        
        if input == "/history" {
            output.info("Conversation history:");
            for (i, msg) in conversation_history.iter().enumerate() {
                println!("  {}: {}: {}", i + 1, msg.role, msg.content);
            }
            continue;
        }
        
        // Add user message to history
        conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: input.to_string(),
            name: None,
        });
        
        // Create chat completion request
        let request = ChatCompletionRequest {
            model: model_name.clone(),
            messages: conversation_history.clone(),
            temperature: Some(cmd.temperature),
            max_tokens: Some(cmd.max_tokens),
            stream: Some(cmd.stream),
            top_p: None,
            n: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            seed: None,
            user: None,
        };
        
        // Send request and get response
        print!("Assistant: ");
        io::stdout().flush().unwrap();
        
        match client.chat_completion(request).await {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    println!("{}", choice.message.content);
                    
                    // Add assistant response to history
                    conversation_history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: choice.message.content.clone(),
                        name: None,
                    });
                    
                    // Show token usage in verbose mode
                    if verbose {
                        if let Some(usage) = response.usage {
                            output.debug(&format!(
                                "Tokens: {} prompt + {} completion = {} total",
                                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                            ));
                        }
                    }
                } else {
                    output.warning("No response generated");
                }
            }
            Err(e) => {
                output.error(&format!("Chat completion failed: {}", e));
                output.info("You can continue the conversation or type 'exit' to quit");
            }
        }
        
        println!(); // Add blank line for readability
    }
    
    // Save conversation history if requested
    if let Some(save_path) = cmd.save_history {
        match save_conversation_history(&conversation_history, &save_path) {
            Ok(_) => output.success(&format!("Conversation saved to: {}", save_path)),
            Err(e) => output.error(&format!("Failed to save conversation: {}", e)),
        }
    }
    
    Ok(())
}

fn save_conversation_history(history: &[ChatMessage], path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(history)
        .context("Failed to serialize conversation history")?;
    
    std::fs::write(path, json)
        .context("Failed to write conversation history to file")?;
    
    Ok(())
}