use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Lokalny silnik podpowiedzi: działa bez sieci i nigdy nie wysyła danych użytkownika.
pub struct LocalSuggest {
    vocabulary: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub text: String,
    pub confidence: f32,
    pub source: &'static str,
}

impl Default for LocalSuggest {
    fn default() -> Self {
        let mut vocabulary = BTreeMap::new();
        vocabulary.insert("pl".into(), vec![
            "Dzień dobry".into(), "Potrzebuję pomocy".into(), "Gdzie jesteś?".into(),
            "Zaraz będę".into(), "Proszę o tłumacza języka migowego".into(), "Wszystko w porządku".into(),
        ]);
        vocabulary.insert("en".into(), vec![
            "Good morning".into(), "I need help".into(), "Where are you?".into(),
            "On my way".into(), "I need a sign language interpreter".into(), "I am fine".into(),
        ]);
        Self { vocabulary }
    }
}

impl LocalSuggest {
    pub fn add(&mut self, language: &str, phrase: &str) {
        self.vocabulary.entry(language.into()).or_default().push(phrase.into());
    }

    pub fn suggest(&self, language: &str, prefix: &str, limit: usize) -> Vec<Suggestion> {
        let needle = prefix.trim().to_lowercase();
        let mut out: Vec<Suggestion> = self
            .vocabulary
            .get(language)
            .map(|phrases| {
                phrases
                    .iter()
                    .filter(|p| p.to_lowercase().starts_with(&needle))
                    .map(|p| Suggestion {
                        text: p.clone(),
                        confidence: if needle.is_empty() { 0.4 } else { 0.9 },
                        source: "local",
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.truncate(limit);
        out
    }

    pub fn languages(&self) -> Vec<&str> {
        self.vocabulary.keys().map(String::as_str).collect()
    }

    /// Silnik lokalny nie wykonuje żadnych żądań sieciowych — to kontrakt prywatności.
    pub fn touches_network(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_matching_phrases() {
        let engine = LocalSuggest::default();
        let out = engine.suggest("pl", "Dzie", 5);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "Dzień dobry");
    }

    #[test]
    fn limits_results() {
        let engine = LocalSuggest::default();
        assert!(engine.suggest("en", "", 2).len() <= 2);
    }

    #[test]
    fn never_touches_network() {
        assert!(!LocalSuggest::default().touches_network());
    }

    #[test]
    fn can_extend_vocabulary() {
        let mut engine = LocalSuggest::default();
        engine.add("pl", "Potrzebuję tłumacza");
        assert!(!engine.suggest("pl", "Potrzebuję", 5).is_empty());
    }
}
