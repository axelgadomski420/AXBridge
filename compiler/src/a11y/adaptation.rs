use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub direction: Direction,
    pub requires_confirmation: bool,
    pub offline_capable: bool,
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Inbound,
    Outbound,
    Bidirectional,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdaptationPlan {
    pub persona: String,
    pub inbound: Vec<Channel>,
    pub outbound: Vec<Channel>,
    pub large_targets: bool,
    pub reduce_motion: bool,
    pub max_actions: usize,
    pub announce_changes: bool,
}

pub fn plan_for(persona: &str) -> AdaptationPlan {
    match persona {
        "deaf" => AdaptationPlan {
            persona: persona.into(),
            inbound: vec![channel("sign", Direction::Inbound, true), channel("text", Direction::Inbound, false)],
            outbound: vec![channel("text", Direction::Outbound, false), channel("visual", Direction::Outbound, false)],
            large_targets: true, reduce_motion: false, max_actions: 6, announce_changes: true,
        },
        "mute" => AdaptationPlan {
            persona: persona.into(),
            inbound: vec![channel("text", Direction::Inbound, false)],
            outbound: vec![channel("speech", Direction::Outbound, true), channel("text", Direction::Outbound, false)],
            large_targets: true, reduce_motion: false, max_actions: 6, announce_changes: true,
        },
        "deafBlind" => AdaptationPlan {
            persona: persona.into(),
            inbound: vec![channel("morse", Direction::Inbound, true)],
            outbound: vec![channel("haptic", Direction::Outbound, false), channel("morse", Direction::Outbound, false)],
            large_targets: true, reduce_motion: true, max_actions: 4, announce_changes: true,
        },
        "blind" => AdaptationPlan {
            persona: persona.into(),
            inbound: vec![channel("speech", Direction::Inbound, true)],
            outbound: vec![channel("speech", Direction::Outbound, false)],
            large_targets: true, reduce_motion: true, max_actions: 5, announce_changes: true,
        },
        "lowVision" => AdaptationPlan {
            persona: persona.into(),
            inbound: vec![channel("text", Direction::Inbound, false)],
            outbound: vec![channel("text", Direction::Outbound, false)],
            large_targets: true, reduce_motion: false, max_actions: 8, announce_changes: true,
        },
        "motor" => AdaptationPlan {
            persona: persona.into(),
            inbound: vec![channel("text", Direction::Inbound, false)],
            outbound: vec![channel("text", Direction::Outbound, false)],
            large_targets: true, reduce_motion: true, max_actions: 3, announce_changes: true,
        },
        "neurodivergent" => AdaptationPlan {
            persona: persona.into(),
            inbound: vec![channel("text", Direction::Inbound, false)],
            outbound: vec![channel("text", Direction::Outbound, false)],
            large_targets: true, reduce_motion: true, max_actions: 4, announce_changes: false,
        },
        _ => AdaptationPlan {
            persona: "general".into(),
            inbound: vec![channel("text", Direction::Inbound, false), channel("speech", Direction::Bidirectional, true)],
            outbound: vec![channel("text", Direction::Outbound, false), channel("speech", Direction::Outbound, false)],
            large_targets: true, reduce_motion: false, max_actions: 8, announce_changes: true,
        },
    }
}

fn channel(id: &str, direction: Direction, requires_confirmation: bool) -> Channel {
    Channel {
        id: id.into(),
        direction,
        requires_confirmation,
        offline_capable: true,
        fallback: match id {
            "speech" => Some("text".into()),
            "sign" => Some("text".into()),
            "morse" => Some("simplified".into()),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deaf_blind_prefers_haptic_morse() {
        let plan = plan_for("deafBlind");
        assert_eq!(plan.inbound[0].id, "morse");
        assert_eq!(plan.outbound[0].id, "haptic");
        assert!(plan.reduce_motion);
    }

    #[test]
    fn motor_persona_limits_actions() {
        let plan = plan_for("motor");
        assert!(plan.max_actions <= 4);
        assert!(plan.large_targets);
    }

    #[test]
    fn every_channel_declares_offline_capability() {
        for persona in ["deaf", "mute", "blind", "lowVision", "motor", "neurodivergent", "general"] {
            let plan = plan_for(persona);
            assert!(plan.inbound.iter().all(|c| c.offline_capable));
        }
    }
}
