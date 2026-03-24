//! Voice tools (STT and TTS).
//!
//! Provides tools for synthesizing speech from text and transcribing
//! audio files into text using OpenAI's Audio API.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;
use sk_types::ToolDefinition;
use std::env;
use std::fs;
use std::path::Path;
use tracing::info;

/// Tool to synthesize speech from text.
pub fn text_to_speech_tool() -> ToolDefinition {
    ToolDefinition {
        name: "text_to_speech".to_string(),
        description: "Generate an audio file from text using AI. Supports multiple voices like 'alloy', 'echo', 'fable', 'onyx', 'nova', and 'shimmer'.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to synthesize into speech."
                },
                "voice": {
                    "type": "string",
                    "description": "The voice to use (default: 'alloy').",
                    "enum": ["alloy", "echo", "fable", "onyx", "nova", "shimmer"]
                },
                "output_path": {
                    "type": "string",
                    "description": "Optional: Specific path to save the .mp3 file. If not provided, a temporary file is used."
                }
            },
            "required": ["text"]
        }),
    }
}

/// Tool to transcribe audio files into text.
pub fn speech_to_text_tool() -> ToolDefinition {
    ToolDefinition {
        name: "speech_to_text".to_string(),
        description: "Transcribe an audio file into text using OpenAI Whisper.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the audio file to transcribe (mp3, mp4, mpeg, mpga, m4a, wav, or webm)."
                }
            },
            "required": ["file_path"]
        }),
    }
}

/// Handle a text_to_speech call.
pub async fn handle_text_to_speech(
    text: &str,
    voice: Option<&str>,
    output_path: Option<&str>,
) -> Result<String, String> {
    let api_key = env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY not found in environment".to_string())?;

    let voice = voice.unwrap_or("alloy");
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap());
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application_json"));

    let body = json!({
        "model": "tts-1",
        "input": text,
        "voice": voice,
    });

    info!("Synthesizing speech for: '{}' (voice: {})", text, voice);

    let response = client
        .post("https://api.openai.com/v1/audio/speech")
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("TTS request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body = response.text().await.unwrap_or_default();
        return Err(format!("TTS API error ({}): {}", status, err_body));
    }

    let bytes = response.bytes().await.map_err(|e| format!("Failed to read TTS body: {}", e))?;

    let path = if let Some(p) = output_path {
        p.to_string()
    } else {
        format!("tts_{}.mp3", uuid::Uuid::new_v4())
    };

    fs::write(&path, bytes).map_err(|e| format!("Failed to write TTS file to {}: {}", path, e))?;

    Ok(format!("Speech synthesized successfully and saved to: {}", path))
}

/// Handle a speech_to_text call.
pub async fn handle_speech_to_text(file_path: &str) -> Result<String, String> {
    let api_key = env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY not found in environment".to_string())?;

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("Audio file not found: {}", file_path));
    }

    let client = reqwest::Client::new();

    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .file("file", file_path)
        .await
        .map_err(|e| format!("Failed to build multipart form: {}", e))?;

    info!("Transcribing audio file: {}", file_path);

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("STT request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body = response.text().await.unwrap_or_default();
        return Err(format!("STT API error ({}): {}", status, err_body));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse STT response: {}", e))?;

    let transcription = result["text"]
        .as_str()
        .ok_or_else(|| "Missing 'text' field in Whisper response".to_string())?;

    Ok(transcription.to_string())
}
