use dotenvy::dotenv;
use log::info;
use serde_json;
use serde_json::Value;
use teloxide::{prelude::*, types::Message as TelegramMessage};
use deepseek_rs::{client::chat_completions::request::{Message, RequestBody}, DeepSeekClient};
use std::env::var as env_var;

#[tokio::main]
async fn main() {
	dotenv().ok();

	pretty_env_logger::init();

	info!("Starting bot...");

	let bot = Bot::from_env();

	teloxide::repl(bot, |_bot: Bot, msg: TelegramMessage| async move {
		let json = match serde_json::to_string_pretty(&msg) {
			Ok(json) => {
				info!("Message as JSON:\n{}", json);
				json
			},
			Err(e) => {
				info!("Failed to serialize msg: {}", e);
				String::new()
			},
		};

		// if the message does not have a text, return
		if msg.text().is_none() {
			info!("Received a message without text");
			return Ok(());
		}

		if msg.chat.is_group() || msg.chat.is_supergroup() {
			info!("Received a message in a group chat");
			// fetch deepseek key from env
			let deepseek_key = env_var("DEEPSEEK_API_KEY").unwrap_or_else(|_| {
				info!("DEEPSEEK_API_KEY not found in env");
				String::new()
			});
			let deepseek_client = DeepSeekClient::new_with_api_key(deepseek_key);
			let system_prompt = "You are an AI assistant responsible for determining whether a chat message is spam or part of illegal gray-market content.\nPlease follow these rules carefully:\n\n- Consider both the message content **and the sender's profile information** (including `first_name`, `last_name`, `username`).\n- Watch out for suspicious profile fields containing phrases like \"loan\", \"click here\", \"add me\", etc.\n- If the profile contains spam indicators but the text is benign, **still classify it as spam**.\n- Do **not** classify messages as spam **just because** they use memes, emojis, exaggerated tone, or mimic spam formatting.\n- A message **should be marked as spam only if it:\n\t- Attempts to redirect users to external websites;\n\t- Promotes illegal services or activities;\n\t- Tries to induce clicks, scan codes, or add unknown contacts.\n- If the message is simply humorous, uses trendy phrases, or follows meme formats **without harmful intent**, classify it as **not spam**.\n\nReturn only in JSON format:\n\n```json\n{ \"is_spam\": true/false }\"".to_string();
			let request = RequestBody::new_messages(vec![
				Message::new_system_message(system_prompt),
				Message::new_user_message(json),
			]);
			let response = deepseek_client.chat_completions(request).await;
			if let Ok(result) = response {
				if let Some(choice) = result.choices.first() {
						if let Some(content) = &choice.message.content {
							info!("🎯 Model replied:\n{}", content);
							if let Some(json) = extract_json_block(content) {
								info!("✅ Parsed JSON: {}", json);
								if let Some(is_spam) = json.get("is_spam") {
									info!("🚨 Spam status: {}", is_spam);
									if json.get("is_spam").and_then(Value::as_bool).unwrap_or(false) {
										// delete the message
										if let Err(e) = _bot.delete_message(msg.chat.id, msg.id).await {
											info!("❌ Failed to delete message: {}", e);
										} else {
											info!("✅ Message deleted successfully.");
										}
									}
								}
							}
						} else {
							info!("⚠️ Model returned a message with no content.");
						}
				} else {
					info!("⚠️ No choices returned by the model.");
				}
			} else {
				info!("❌ Request to DeepSeek failed: {:?}", response);
			}
		}

		Ok(())
	})
	.await;
}

fn extract_json_block(text: &str) -> Option<Value> {
	let pure_json_string = text.trim()
		.trim_start_matches("```json")
		.trim_start_matches("```")
		.trim_end_matches("```");

	serde_json::from_str::<Value>(pure_json_string).ok()
}