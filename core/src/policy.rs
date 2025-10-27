use serde::{Deserialize, Serialize};

/// High level banner content presented in the UI to surface redistribution rules
/// before any translation workflow begins.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolicyBanner {
    pub headline: String,
    pub message: String,
    pub requires_acknowledgement: bool,
    pub checkbox_label: String,
    pub warning: String,
}

/// Captures the restrictions that may apply to a specific game or mod line.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolicyProfile {
    pub game: String,
    pub redistribution_blocked: bool,
    pub requires_author_permission: bool,
    pub eula_reference: Option<String>,
    pub notes: Vec<String>,
}

impl PolicyProfile {
    /// Produce a conservative default profile that errs on the side of caution.
    pub fn conservative(game: impl Into<String>) -> Self {
        let game = game.into();
        let mut notes = vec![
            "개인 백업과 로컬 플레이는 허용되지만, 재배포를 위해서는 제작자의 동의가 필요합니다."
                .to_string(),
            "수정된 콘텐츠를 내보낼 때는 Steam 워크샵 가이드라인을 준수하세요.".to_string(),
        ];

        if game.to_ascii_lowercase().contains("skyrim") {
            notes.push(
                "Creation Club 및 Bethesda.net 약관이 적용될 수 있습니다. 재배포 규정은 https://bethesda.net 에서 확인하세요.".into(),
            );
        }

        Self {
            game,
            redistribution_blocked: true,
            requires_author_permission: true,
            eula_reference: Some(
                "게임의 최종 사용자 라이선스(EULA)에서 현지화 콘텐츠 관련 조항을 확인하세요."
                    .into(),
            ),
            notes,
        }
    }
}

pub fn default_policy_banner() -> PolicyBanner {
    PolicyBanner {
        headline: "개인용으로만 사용".into(),
        message: "Steam 워크샵 자산은 개인 이용 목적으로만 제공됩니다. 번역된 빌드를 다시 배포하려면 원 제작자의 허가가 필요합니다.".into(),
        requires_acknowledgement: true,
        checkbox_label: "재배포에는 명시적인 허락이 필요함을 이해했습니다.".into(),
        warning: "Steam/커뮤니티 이용 규정을 준수하세요. 위반 시 계정 제한이 발생할 수 있으며, 각 게임의 EULA 조항을 확인해야 합니다.".into(),
    }
}
