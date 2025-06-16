use serde::{Deserialize, Serialize};

#[tauri::command]
fn greet(name: &str) -> String {
  format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![greet])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[derive(Debug, Serialize)]
pub struct ChatRequest<'a> {
  pub model: &'a str,
  pub messages: &'a [Message<'a>],
}

#[derive(Debug, Serialize)]
pub struct Message<'a> {
  pub role: &'a str,
  pub content: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
  pub id: String,
  pub object: String,
  pub created: u64,
  pub choices: Vec<Choice>,
  pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
  pub index: u32,
  pub message: MessageResponse,
  pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageResponse {
  pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
  pub prompt_tokens: u32,
  pub completion_tokens: u32,
  pub total_tokens: u32,
}
