use serde::Serialize;

/// High level banner content presented in the UI to surface redistribution rules
/// before any translation workflow begins.
#[derive(Debug, Serialize, Clone)]
pub struct PolicyBanner {
    pub headline: String,
    pub message: String,
    pub requires_acknowledgement: bool,
    pub checkbox_label: String,
    pub warning: String,
}

/// Captures the restrictions that may apply to a specific game or mod line.
#[derive(Debug, Serialize, Clone)]
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
            "Personal backups and local play are supported; redistribution requires consent."
                .to_string(),
            "Respect Steam Workshop guidelines when exporting any modified content.".to_string(),
        ];

        if game.to_ascii_lowercase().contains("skyrim") {
            notes.push(
                "Creation Club and Bethesda.net terms may apply; consult https://bethesda.net for redistribution rules.".into(),
            );
        }

        Self {
            game,
            redistribution_blocked: true,
            requires_author_permission: true,
            eula_reference: Some(
                "Check the game's End User License Agreement for localized content provisions."
                    .into(),
            ),
            notes,
        }
    }
}

pub fn default_policy_banner() -> PolicyBanner {
    PolicyBanner {
        headline: "Personal-use Only".into(),
        message: "Steam Workshop assets are provided for personal use. Re-uploading localized builds requires the original author's permission.".into(),
        requires_acknowledgement: true,
        checkbox_label: "I understand that redistribution requires explicit permission.".into(),
        warning: "Steam/Community Guidelines: Violations may result in account restrictions. Always comply with game-specific EULAs.".into(),
    }
}
