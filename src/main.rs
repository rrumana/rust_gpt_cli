// main.rs

use clap::Parser;
use chrono::Utc;
use reqwest::header::{CONTENT_TYPE, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::Write;
use tokio::io::{self, AsyncBufReadExt, BufReader};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The model to use for the conversation (default: gpt-3.5-turbo)
    #[arg(short, long, default_value = "gpt-3.5-turbo")]
    model: String,
    /// Enable debug mode to generate additional files for testing.
    #[arg(short, long, action)]
    debug: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Deserialize, Debug)]
struct ChatChoice {
    message: Message,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

async fn update_summary(
    client: &reqwest::Client,
    api_key: &str,
    current_summary: Option<&str>,
    messages: &[Message],
) -> Result<String, Box<dyn Error>> {
    let system_msg = Message {
        role: "system".to_string(),
        content: "You are a helpful assistant tasked with summarizing a conversation. Try to keep the summary short, but make sure to include each relevant bullet point. I would rather you make the summary longer than forget things.:w
.".to_string(),
    };

    let mut user_content = String::new();
    if let Some(summary) = current_summary {
        user_content.push_str("Current summary:\n");
        user_content.push_str(summary);
        user_content.push_str("\n\nAdd the following new exchange to the summary, try your hardest to retain as much information as possible:\n");
    } else {
        user_content.push_str("Summarize the following conversation in under 200 words:\n");
    }
    for msg in messages {
        user_content.push_str(&format!("{}: {}\n", msg.role, msg.content));
    }
    user_content.push_str("\nPlease provide an updated summary.");

    let request_body = ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![
            system_msg,
            Message {
                role: "user".to_string(),
                content: user_content,
            },
        ],
    };

    let url = "https://api.openai.com/v1/chat/completions";
    let res = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if res.status().is_success() {
        let chat_response: ChatResponse = res.json().await?;
        if let Some(choice) = chat_response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err("No summary returned by GPT-4o".into())
        }
    } else {
        let error_text = res.text().await?;
        Err(format!("Error summarizing: {}", error_text).into())
    }
}

fn build_context(conversation: &[Message], summary: &Option<String>) -> Vec<Message> {
    let total_exchanges = conversation.len() / 2;
    if total_exchanges <= 10 {
        conversation.to_vec()
    } else {
        let start_index = conversation.len() - 20;
        let mut context = Vec::new();
        if let Some(sum) = summary {
            context.push(Message {
                role: "system".to_string(),
                content: sum.clone(),
            });
        }
        context.extend_from_slice(&conversation[start_index..]);
        context
    }
}

/// Debug function: writes a fiile to track the current context 
/// - "debug_context.txt" contains the context prompt (summary and the last few messages).
fn save_debug_files(conversation: &[Message], summary: &Option<String>) -> Result<(), Box<dyn Error>> {
    let context = build_context(conversation, summary);
    let mut ctx_file = std::fs::File::create("debug_context.txt")?;
    writeln!(ctx_file, "Context Prompt:")?;
    for msg in &context {
        writeln!(ctx_file, "{}: {}", msg.role, msg.content)?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable not set");

    let client = reqwest::Client::new();
    let url = "https://api.openai.com/v1/chat/completions";

    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    let mut conversation: Vec<Message> = Vec::new();
    let mut summary: Option<String> = None;

    println!(
        "Interactive Chat Session (model: {}). Type your message below. Press Ctrl+C to exit.\n",
        args.model
    );

    loop {
        tokio::select! {
            maybe_line = lines.next_line() => {
                match maybe_line? {
                    Some(input) => {
                        let prompt = input.trim();
                        if prompt.is_empty() {
                            continue;
                        }
                        conversation.push(Message {
                            role: "user".to_string(),
                            content: prompt.to_string(),
                        });

                        let context_messages = build_context(&conversation, &summary);

                        let request_body = ChatRequest {
                            model: args.model.clone(),
                            messages: context_messages,
                        };

                        let res = client.post(url)
                            .header(CONTENT_TYPE, "application/json")
                            .header(AUTHORIZATION, format!("Bearer {}", api_key))
                            .json(&request_body)
                            .send()
                            .await?;

                        if res.status().is_success() {
                            let chat_response: ChatResponse = res.json().await?;
                            if let Some(choice) = chat_response.choices.first() {
                                let reply = &choice.message.content;
                                println!("{}: {}\n", args.model, reply);
                                conversation.push(Message {
                                    role: "assistant".to_string(),
                                    content: reply.to_string(),
                                });
                            } else {
                                eprintln!("No response returned by the API.");
                            }
                        } else {
                            let error_text = res.text().await?;
                            eprintln!("Error: {}", error_text);
                        }

                        if conversation.len() / 2 > 10 {
                            if summary.is_none() {
                                // Create the initial summary from all messages before the last 5 exchanges.
                                let summary_source = &conversation[..conversation.len()-20];
                                summary = Some(update_summary(&client, &api_key, None, summary_source).await?);
                            } else {
                                // Update the summary with the latest exchange (last two messages).
                                let new_exchange = &conversation[conversation.len()-22..conversation.len()-20];
                                summary = Some(update_summary(&client, &api_key, summary.as_deref(), new_exchange).await?);
                            }
                        }

                        if args.debug {
                            if let Err(e) = save_debug_files(&conversation, &summary) {
                                eprintln!("Debug file error: {}", e);
                            }
                        }
                    },
                    None => break,
                }
            },
            _ = tokio::signal::ctrl_c() => {
                println!("\nTermination signal received.");
                break;
            }
        }
    }

    if args.debug {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let transcript_file = format!("chat_transcript_{}.txt", timestamp);
        let mut file = std::fs::File::create(transcript_file)?;
        writeln!(file, "Conversation Transcript:")?;
        for msg in conversation.iter() {
            writeln!(file, "{}: {}", msg.role, msg.content)?;
        }

        if let Err(e) = save_debug_files(&conversation, &summary) {
            eprintln!("Final debug file error: {}", e);
        } else {
            println!("Debug files 'chat_transcription.txt' and generated. Press enter to continue");
        }
    }

    Ok(())
}
