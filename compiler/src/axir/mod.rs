//! AXIR-2 – Advanced Intermediate Representation
//! Platform-neutral graph of screens, state, music, translation, providers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxirModule {
    pub name: String,
    pub version: String,
    pub screens: Vec<ScreenNode>,
    pub components: Vec<ComponentNode>,
    pub stores: Vec<StoreNode>,
    pub functions: Vec<FunctionNode>,
    pub providers: Vec<ProviderNode>,
    pub persona_defaults: HashMap<String, PersonaConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenNode {
    pub name: String,
    pub state: Vec<StateNode>,
    pub body: Vec<UiNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentNode {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub body: Vec<UiNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreNode {
    pub name: String,
    pub kind: String,
    pub data_type: String,
    pub key: String,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionNode {
    pub name: String,
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderNode {
    pub id: String,
    pub name: String,
    pub ty: String, // llm | music | translate | stt | tts
    pub enabled: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaConfig {
    pub preferred_input: String,
    pub preferred_output: String,
    pub large_targets: bool,
    pub high_contrast: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateNode {
    pub name: String,
    pub ty: String,
    pub initial: Option<String>,
    pub is_global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiNode {
    Column(Vec<UiNode>),
    Row(Vec<UiNode>),
    Text { value: String },
    Button { label: String, action: Option<String> },
    List { source: String, item: Box<UiNode> },
    MusicPlayer { meta_ref: String },
    TranslateBadge,
    Custom(String),
}
