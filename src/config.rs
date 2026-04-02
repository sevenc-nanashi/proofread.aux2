use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Gemini,
    ChatGpt,
    Claude,
    OpenRouter,
}

impl Default for Preset {
    fn default() -> Self {
        Self::Gemini
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    pub preset: Preset,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

impl Default for Credentials {
    fn default() -> Self {
        let defaults = Preset::Gemini.defaults();
        Self {
            preset: Preset::Gemini,
            base_url: defaults.base_url.to_string(),
            model: defaults.model.to_string(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetDefaults {
    pub base_url: &'static str,
    pub model: &'static str,
    pub key_url: &'static str,
}

impl Preset {
    pub fn defaults(self) -> PresetDefaults {
        match self {
            Self::Gemini => PresetDefaults {
                base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
                model: "gemini-2.5-flash",
                key_url: "https://aistudio.google.com/app/apikey",
            },
            Self::ChatGpt => PresetDefaults {
                base_url: "https://api.openai.com/v1",
                model: "gpt-4.1-mini",
                key_url: "https://platform.openai.com/api-keys",
            },
            Self::Claude => PresetDefaults {
                base_url: "https://api.anthropic.com/v1/",
                model: "claude-3-7-sonnet-latest",
                key_url: "https://console.anthropic.com/settings/keys",
            },
            Self::OpenRouter => PresetDefaults {
                base_url: "https://openrouter.ai/api/v1",
                model: "openai/gpt-4.1-mini",
                key_url: "https://openrouter.ai/keys",
            },
        }
    }
}

pub fn credentials_path() -> std::io::Result<PathBuf> {
    let local = env::var_os("LOCALAPPDATA").ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "LOCALAPPDATA is not set")
    })?;
    Ok(Path::new(&local)
        .join("proofread.aux2")
        .join("credentials.json"))
}

pub fn load_credentials(path: &Path) -> std::io::Result<Credentials> {
    let raw = fs::read_to_string(path)?;
    let creds: Credentials = serde_json::from_str(&raw).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to parse credentials.json: {err}"),
        )
    })?;
    Ok(creds)
}

pub fn save_credentials(path: &Path, credentials: &Credentials) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(credentials).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to serialize credentials: {err}"),
        )
    })?;
    fs::write(path, raw)
}

#[cfg(test)]
mod tests {
    use super::{Credentials, Preset};

    #[test]
    fn gemini_is_default_preset() {
        let credentials = Credentials::default();
        assert_eq!(credentials.preset, Preset::Gemini);
        assert!(credentials.base_url.contains("googleapis"));
    }

    #[test]
    fn all_presets_have_non_empty_defaults() {
        let presets = [
            Preset::Gemini,
            Preset::ChatGpt,
            Preset::Claude,
            Preset::OpenRouter,
        ];

        for preset in presets {
            let defaults = preset.defaults();
            assert!(!defaults.base_url.is_empty());
            assert!(!defaults.model.is_empty());
            assert!(defaults.key_url.starts_with("https://"));
        }
    }
}
