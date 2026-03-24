---
name: voice
description: Synthesis and transcription of speech.
homepage: https://openai.com/research/whisper
metadata:
  version: 1.0.0
  category: multimedia
---

# Voice Capability

The Voice capability allows agents to interact with the world through sound. It provides advanced Text-to-Speech (TTS) and Speech-to-Text (STT) powered by OpenAI.

## Available Tools

### 1. text_to_speech
Converts written text into high-quality spoken audio.
- **Parameters**:
  - `text`: The message to speak.
  - `voice`: (Optional) 'alloy', 'echo', 'fable', 'onyx', 'nova', or 'shimmer'.
  - `output_path`: (Optional) Where to save the resulting .mp3 file.
- **Usage**: Use this to "speak" to the user or generate audio content for other tools.

### 2. speech_to_text
Transcribes audio files into written text.
- **Parameters**:
  - `file_path`: Path to an audio file (mp3, wav, etc.).
- **Usage**: Use this to process voice notes, meeting recordings, or any audio input.

## Guidelines
- Always ensure `OPENAI_API_KEY` is set in the environment.
- When generating speech, favor the 'alloy' or 'nova' voices for neutral, clear synthesis.
- Transcription works best with clear audio and supports multiple languages automatically.
