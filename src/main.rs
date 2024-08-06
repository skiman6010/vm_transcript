use dotenv::dotenv;
use serde::Deserialize;
use teloxide::{prelude::*, types::File as TelegramFile};
use tokio::{fs, task};
use reqwest::multipart;
use serde_json::Value;
use std::path::Path;

#[derive(Deserialize, Debug, Clone)]
struct Config {
    bot_token: String,
    asr_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let config = envy::from_env::<Config>()?;

    // Create 'downloads' directory if it doesn't exist
    if !Path::new("downloads").exists() {
        std::fs::create_dir("downloads")?;
        println!("Created 'downloads' directory");
    }

    let bot = Bot::new(&config.bot_token);

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let config = config.clone();
        async move {
            if let Some(voice) = msg.voice() {
                println!("Received a voice recording!");
                if let Err(e) = bot.send_message(msg.chat.id, "Received a voice recording!").await {
                    eprintln!("Failed to send message: {}", e);
                    return Ok(());
                }

                let file: TelegramFile = match bot.get_file(&voice.file.id).await {
                    Ok(file) => file,
                    Err(e) => {
                        eprintln!("Failed to get file: {}", e);
                        return Ok(());
                    }
                };
                let file_path = file.path;

                // Download the file using the file_path
                let download_url = format!("https://api.telegram.org/file/bot{}/{}", config.bot_token, file_path);
                let response = match reqwest::get(&download_url).await {
                    Ok(response) => response,
                    Err(e) => {
                        eprintln!("Failed to download file: {}", e);
                        return Ok(());
                    }
                };
                let bytes = match response.bytes().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        eprintln!("Failed to get bytes: {}", e);
                        return Ok(());
                    }
                };

                let file_name = format!("downloads/{}.ogg", voice.file.unique_id);
                if let Err(e) = fs::write(&file_name, &bytes).await {
                    eprintln!("Failed to write file: {}", e);
                    return Ok(());
                }

                // Send file to ASR service
                let client = reqwest::Client::new();

                let file_content = match fs::read(&file_name).await {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!("Failed to read file: {}", e);
                        return Ok(());
                    }
                };
                let file_part = multipart::Part::bytes(file_content)
                    .file_name(file_name.clone());
                let form = multipart::Form::new()
                    .part("audio_file", file_part);

                let response = match client.post(&config.asr_url)
                    .multipart(form)
                    .timeout(std::time::Duration::from_secs(600))
                    .send()
                    .await {
                    Ok(response) => response,
                    Err(e) => {
                        eprintln!("Failed to send request to ASR service: {}", e);
                        return Ok(());
                    }
                };

                let response_text = match response.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        eprintln!("Failed to get response text: {}", e);
                        return Ok(());
                    }
                };
                println!("{}", response_text);

                let json: Value = match serde_json::from_str(&response_text) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("Failed to parse JSON: {}", e);
                        return Ok(());
                    }
                };
                let whisper_text = json["text"].as_str().unwrap_or("No text found");

                if let Err(e) = bot.send_message(msg.chat.id, whisper_text).await {
                    eprintln!("Failed to send message: {}", e);
                }

                // Clean up the downloaded file
                task::spawn(async move {
                    if let Err(e) = fs::remove_file(file_name).await {
                        eprintln!("Failed to remove file: {}", e);
                    }
                });
            }

            Ok(())
        }
    })
        .await;

    Ok(())
}