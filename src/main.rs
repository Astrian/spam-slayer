use dotenvy::dotenv;
use gemini_client_rs::{
	types::{Content, ContentPart, GenerateContentRequest, PartResponse, Role},
	GeminiClient,
};
use log::info;
use serde_json::{self, json, Value};
use std::env::var as env_var;
use teloxide::{prelude::*, types::Message as TelegramMessage};
use uuid::Uuid;
use std::fs::OpenOptions;
use std::io::Write;

#[tokio::main]
async fn main() {
	dotenv().ok();

	pretty_env_logger::init();

	info!("Starting bot...");

	let bot = Bot::from_env();

	teloxide::repl(bot, |_bot: Bot, msg: TelegramMessage| async move {
		let msg_json = match serde_json::to_string_pretty(&msg) {
				Ok(json) => {
					info!("Message as JSON:\n{}", json);
					json
				}
				Err(e) => {
				info!("Failed to serialize msg: {}", e);
				String::new()
			}
		};

		// if the message does not have a text, return
		if msg.text().is_none() {
			info!("Received a message without text");
			return Ok(());
		}

		if msg.chat.is_group() || msg.chat.is_supergroup() {
			info!("Received a message in a group chat");

			// detect is_automatic_forward
			if msg.is_automatic_forward() {
				info!("Received a message from a forward. Will be ignored.");
				// if the message is from a forward, return
				return Ok(());
			}

			// fetch deepseek key from env
			let gemini_key = env_var("GEMINI_API_KEY").unwrap_or_else(|_| {
				info!("GEMINI_API_KEY not found in env");
				String::new()
			});

			// create a client
			let client = GeminiClient::new(gemini_key);
			let model_name = "gemini-2.5-flash-preview-04-17";

			// create a request
			let system_prompt = "You are an AI assistant responsible for determining whether a chat message is spam or part of illegal gray-market content.\nPlease follow these rules carefully:\n\n- Consider both the message content **and the sender's profile information** (including `first_name`, `last_name`, `username`).\n- Watch out for suspicious profile fields containing phrases like \"loan\", \"click here\", \"add me\", etc.\n- If the profile contains spam indicators but the text is benign, **still classify it as spam**.\n- Do **not** classify messages as spam **just because** they use memes, emojis, exaggerated tone, or mimic spam formatting.\n- A message **should be marked as spam only if it:\n\t- Attempts to redirect users to external websites;\n\t- Promotes illegal services or activities;\n\t- Tries to induce clicks, scan codes, or add unknown contacts.\n- If the message is simply humorous, uses trendy phrases, or follows meme formats **without harmful intent**, classify it as **not spam**.\n\nReturn only in JSON format:\n\n```json\n{ \"is_spam\": true/false }\"".to_string();
			let mut history: Vec<Content> = vec![];
			history.push(Content {
				role: Role::User,
				parts: vec![ContentPart::Text(system_prompt)],
			});
			history.push(Content {
				role: Role::User,
				parts: vec![ContentPart::Text(msg_json.clone())],
			});
			let req_json = json!(
				{
					"contents": history,
				}
			);

			// print the message
			let request: GenerateContentRequest =
				serde_json::from_value(req_json).expect("Invalid JSON");
			let response = match client.generate_content(model_name, &request).await {
					Ok(response) => response,
					Err(e) => {
					info!("Error: {:?}", e);
					return Ok(());
				}
			};
			if let Some(candidates) = response.candidates {
				for candidate in &candidates {
					for part in &candidate.content.parts {
						match part {
							PartResponse::Text(text) => {
								if let Some(json) = extract_json_block(text) {
									info!("Parsed JSON: {}", json);
									if let Some(content) = json.get("is_spam") {
										info!("Spam status: {}", content);
										let is_spam = json
											.get("is_spam")
											.and_then(Value::as_bool)
											.unwrap_or(false);

										// Generate UUID
										let uuid = Uuid::new_v4();

										// Write to CSV file
										if let Err(e) = write_to_csv(msg_json.clone(), is_spam, &uuid.to_string()).await {
											info!("Failed to write to CSV: {}", e);
										}

										if is_spam
										{
											// delete the message
											if let Err(e) =
												_bot.delete_message(msg.chat.id, msg.id).await
											{
												info!("Failed to delete message: {}", e);
											} else {
												info!("Message deleted successfully.");
											}
										}
									} else {
										info!("No 'is_spam' field found in JSON");
									}
								} else {
										info!("No JSON found in text: {}", text);
								}
							}
							_ => {}
						}
					}
				}
			}
		}

		Ok(())
	})
	.await;
}

async fn write_to_csv(message_json: String, is_spam: bool, uuid: &str) -> Result<(), Box<dyn std::error::Error>> {
	info!("Writing to CSV file...");
	let file_path = "./samples.csv";
	let mut file = OpenOptions::new()
		.write(true)
		.append(true)
		.create(true)
		.open(file_path)?;

	let message_text = message_json.to_string();
	let csv_line = format!("\"{}\",\"{}\",\"{}\"\n", message_text.replace("\"", "\"\"").replace("\n", "").replace(" ", ""), is_spam, uuid);

	file.write_all(csv_line.as_bytes())?;
	Ok(())
}

fn extract_json_block(text: &str) -> Option<Value> {
	let pure_json_string = text
		.trim()
		.trim_start_matches("```json")
		.trim_start_matches("```")
		.trim_end_matches("```");

	serde_json::from_str::<Value>(pure_json_string).ok()
}
