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

/// Wiersz matrycy zdolności kanałów — jedna prawda o tym, co wymaga człowieka.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityRow {
    pub channel: String,
    pub kind: String,
    pub requires_confirmation: bool,
    pub offline_capable: bool,
    pub redundant_representation: bool,
}

/// Pełna matryca kanałów: 11 kopert, ich zdolność offline, potrzeba potwierdzenia
/// i obecność reprezentacji równoważnej.
pub fn capability_matrix() -> Vec<CapabilityRow> {
    let samples = vec![
        Payload::Text { body: String::new() },
        Payload::Speech { transcript: String::new(), confidence: 0.0, audio_key: String::new() },
        Payload::Morse { code: String::new(), wpm: 15 },
        Payload::Sign { glosses: vec![], confidence: 0.0, video_key: None },
        Payload::Music { prompt: String::new(), provider: String::new(), duration_ms: 0, r2_key: String::new() },
        Payload::Image { media_key: String::new(), alt: String::new() },
        Payload::Video { media_key: String::new(), captions: None, alt: String::new() },
        Payload::Audio { media_key: String::new(), transcript: None },
        Payload::Location { latitude: 0.0, longitude: 0.0, precision: Precision::Approximate },
        Payload::Alarm { message: String::new(), expires_at: String::new(), recipients: vec![] },
        Payload::Reaction { mood: Mood::Calm },
    ];
    samples
        .iter()
        .map(|payload| CapabilityRow {
            channel: payload.audit_label().to_string(),
            kind: channel_kind(payload).to_string(),
            requires_confirmation: payload.requires_confirmation(&[]),
            offline_capable: payload.is_offline_capable(),
            redundant_representation: payload.redundant_representation().is_some(),
        })
        .collect()
}

fn channel_kind(payload: &Payload) -> &'static str {
    match payload {
        Payload::Text { .. } => "base",
        Payload::Speech { .. } => "ai-transcribed",
        Payload::Morse { .. } => "encoded",
        Payload::Sign { .. } => "ai-recognised",
        Payload::Music { .. } => "ai-generated",
        Payload::Image { .. } => "media",
        Payload::Video { .. } => "media",
        Payload::Audio { .. } => "media",
        Payload::Location { .. } => "sensitive",
        Payload::Alarm { .. } => "safety-critical",
        Payload::Reaction { .. } => "affective",
    }
}

pub fn aesthetic_metadata(payload: &Payload) -> Vec<(&'static str, String)> {
    let mut meta = vec![("channel", payload.audit_label().to_string())];
    if let Some(redundant) = payload.redundant_representation() {
        meta.push(("redundant", redundant));
    }
    meta.push(("offline", payload.is_offline_capable().to_string()));
    meta.push(("kind", channel_kind(payload).to_string()));
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

    #[test]
    fn matrix_covers_every_channel() {
        let matrix = capability_matrix();
        assert_eq!(matrix.len(), 11);
        assert!(matrix.iter().all(|row| !row.channel.is_empty()));
        assert!(matrix.iter().any(|row| row.channel == "text" && !row.requires_confirmation));
        assert!(matrix.iter().any(|row| row.channel == "sign" && row.requires_confirmation));
        assert!(matrix.iter().any(|row| row.channel == "music" && !row.offline_capable));
    }

    #[test]
    fn aesthetic_metadata_is_complete() {
        let meta = aesthetic_metadata(&speech());
        assert!(meta.iter().any(|(key, _)| *key == "redundant"));
        assert!(meta.iter().any(|(key, _)| *key == "kind"));
    }
}
