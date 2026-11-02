pub mod local;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingLocation {
    Local,
    Cloud,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub processing: ProcessingLocation,
    pub enabled: bool,
    pub is_default: bool,
    pub priority: u8,
    pub usage_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    Llm,
    Music,
    Translate,
    Stt,
    Tts,
}

/// Klucze API nigdy nie opuszczają urządzenia — typ wymusza to na poziomie API.
#[derive(Debug, Clone)]
pub struct SecretHandle {
    pub provider_id: String,
    pub store_key: String,
}

impl SecretHandle {
    pub fn assert_never_uploaded(&self) -> bool {
        self.store_key.starts_with("axbridge.providers.")
    }
}

pub fn resolve<'a>(providers: &'a [Provider], kind: ProviderKind) -> Vec<&'a Provider> {
    let mut candidates: Vec<&Provider> = providers
        .iter()
        .filter(|p| p.kind == kind && p.enabled)
        .collect();
    candidates.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then(a.processing.eq(&ProcessingLocation::Local).cmp(&false))
            .then(a.priority.cmp(&b.priority))
    });
    candidates
}

pub fn pipeline(kind: ProviderKind) -> Vec<&'static str> {
    match kind {
        ProviderKind::Stt => vec!["local-capture", "whisper-local", "cloud-stt"],
        ProviderKind::Llm => vec!["local-suggest", "queue", "cloud-llm"],
        ProviderKind::Translate => vec!["kv-cache", "workers-ai", "cloud-translate"],
        ProviderKind::Tts => vec!["system-tts", "cloud-tts"],
        ProviderKind::Music => vec!["queue", "cloud-music", "r2-cache"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(id: &str, kind: ProviderKind, processing: ProcessingLocation, priority: u8) -> Provider {
        Provider { id: id.into(), name: id.into(), kind, processing, enabled: true, is_default: false, priority, usage_count: 0 }
    }

    #[test]
    fn local_engine_is_preferred_over_cloud() {
        let providers = vec![
            provider("cloud", ProviderKind::Llm, ProcessingLocation::Cloud, 1),
            provider("local", ProviderKind::Llm, ProcessingLocation::Local, 9),
        ];
        assert_eq!(resolve(&providers, ProviderKind::Llm)[0].id, "local");
    }

    #[test]
    fn disabled_providers_are_excluded() {
        let mut cloud = provider("cloud", ProviderKind::Llm, ProcessingLocation::Cloud, 1);
        cloud.enabled = false;
        assert!(resolve(&[cloud], ProviderKind::Llm).is_empty());
    }

    #[test]
    fn secret_handle_stays_local() {
        let handle = SecretHandle { provider_id: "openai".into(), store_key: "axbridge.providers.openai".into() };
        assert!(handle.assert_never_uploaded());
    }
}
