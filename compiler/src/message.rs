use serde::{Deserialize, Serialize};

/// Wiadomość wielokanałowa: jeden model dla tekstu, mowy, Morse’a, migu, muzyki i alarmu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub channel: Payload,
    pub ciphertext: String,
    pub nonce: String,
    pub media_key: Option<String>,
    pub reply_to: Option<String>,
    pub client_timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Payload {
    Text { body: String },
    Speech { transcript: String, confidence: f32, audio_key: String },
    Morse { code: String, wpm: u16 },
    Sign { glosses: Vec<String>, confidence: f32, video_key: Option<String> },
    Music { prompt: String, provider: String, duration_ms: u32, r2_key: String },
    Image { media_key: String, alt: String },
    Video { media_key: String, captions: Option<String>, alt: String },
    Audio { media_key: String, transcript: Option<String> },
    Location { latitude: f64, longitude: f64, precision: Precision },
    Alarm { message: String, expires_at: String, recipients: Vec<String> },
    Reaction { mood: Mood },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precision {
    Exact,
    Approximate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mood {
    Happy,
    Calm,
    Energetic,
    Sad,
    Funny,
    Love,
    Angry,
}

impl Payload {
    /// Kanały, których treść tworzy AI, wymagają zatwierdzenia przez człowieka.
    pub fn requires_confirmation(&self, auto_channels: &[&str]) -> bool {
        match self {
            Payload::Speech { .. } => !auto_channels.contains(&"speech"),
            Payload::Sign { .. } => !auto_channels.contains(&"sign"),
            Payload::Music { .. } => !auto_channels.contains(&"music"),
            Payload::Morse { .. } => !auto_channels.contains(&"morse"),
            _ => false,
        }
    }

    /// Minimalna reprezentacja równoważna dla osób, które nie odbierają danego kanału.
    pub fn redundant_representation(&self) -> Option<String> {
        match self {
            Payload::Speech { transcript, .. } => Some(transcript.clone()),
            Payload::Sign { glosses, .. } => Some(glosses.join(" ")),
            Payload::Music { prompt, .. } => Some(format!("Mood track: {prompt}")),
            Payload::Video { captions, .. } => captions.clone(),
            Payload::Audio { transcript, .. } => transcript.clone(),
            Payload::Image { alt, .. } => Some(alt.clone()),
            _ => None,
        }
    }

    pub fn is_offline_capable(&self) -> bool {
        !matches!(self, Payload::Music { .. } | Payload::Video { .. })
    }

    pub fn audit_label(&self) -> &'static str {
        match self {
            Payload::Text { .. } => "text",
            Payload::Speech { .. } => "speech",
            Payload::Morse { .. } => "morse",
            Payload::Sign { .. } => "sign",
            Payload::Music { .. } => "music",
            Payload::Image { .. } => "image",
            Payload::Video { .. } => "video",
            Payload::Audio { .. } => "audio",
            Payload::Location { .. } => "location",
            Payload::Alarm { .. } => "alarm",
            Payload::Reaction { .. } => "reaction",
        }
    }
}

pub fn aesthetic_metadata(payload: &Payload) -> Vec<(&'static str, String)> {
    let mut meta = vec![("channel", payload.audit_label().to_string())];
    if let Some(redundant) = payload.redundant_representation() {
        meta.push(("redundant", redundant));
    }
    meta.push(("offline", payload.is_offline_capable().to_string()));
    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speech() -> Payload {
        Payload::Speech { transcript: "Dzień dobry".into(), confidence: 0.91, audio_key: "k".into() }
    }

    #[test]
    fn ai_channels_require_human_confirmation() {
        assert!(speech().requires_confirmation(&[]));
        assert!(!speech().requires_confirmation(&["speech"]));
    }

    #[test]
    fn text_never_requires_confirmation() {
        let payload = Payload::Text { body: "hej".into() };
        assert!(!payload.requires_confirmation(&[]));
    }

    #[test]
    fn every_ai_channel_has_a_redundant_representation() {
        assert_eq!(speech().redundant_representation().as_deref(), Some("Dzień dobry"));
        let sign = Payload::Sign { glosses: vec!["DZIEŃ".into(), "DOBRY".into()], confidence: 0.7, video_key: None };
        assert_eq!(sign.redundant_representation().as_deref(), Some("DZIEŃ DOBRY"));
    }

    #[test]
    fn alarm_is_offline_capable() {
        let alarm = Payload::Alarm { message: "Pomoc".into(), expires_at: "2026-11-02T12:00:00Z".into(), recipients: vec![] };
        assert!(alarm.is_offline_capable());
    }
}
