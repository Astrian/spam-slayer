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
			let request = RequestBody::new_messages(vec![
				Message::new_system_message("Here is a message received from a group chat, including the metadata with sender's profile and more. You need to judge if it is a spam or not, including any user-visible metadata. Return a pure, top-level JSON to make sure your output can be parsed by JSON parser. The JSON format: { \"is_spam\": boolean }.".to_string()),
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