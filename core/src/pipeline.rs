use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct PipelinePlan {
    pub target: String,
    pub stages: Vec<PipelineStage>,
    pub validators: Vec<ValidatorSpec>,
    pub skip_rules: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PipelineStage {
    pub name: String,
    pub description: String,
    pub strategy: StageStrategy,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StageStrategy {
    Enumerate,
    Detect,
    Parse,
    Translate,
    Validate,
    Repackage,
}

#[derive(Debug, Serialize, Clone)]
pub struct ValidatorSpec {
    pub name: String,
    pub description: String,
}

impl PipelinePlan {
    pub fn default_for(mod_name: &str) -> Self {
        let target = format!("Translation pipeline for {mod_name}");
        let stages = vec![
            PipelineStage {
                name: "Unpack workshop archive".into(),
                description: "Extracts compressed uploads to a temp workspace for deterministic processing.".into(),
                strategy: StageStrategy::Enumerate,
            },
            PipelineStage {
                name: "Detect file formats".into(),
                description:
                    "Inspects extensions, magic bytes, and sample contents to classify text vs. binary assets.".into(),
                strategy: StageStrategy::Detect,
            },
            PipelineStage {
                name: "Parse resources".into(),
                description:
                    "Loads JSON, INI, XML, CSV, and .resx resources to isolate user-visible key/value content.".into(),
                strategy: StageStrategy::Parse,
            },
            PipelineStage {
                name: "Translate batches".into(),
                description:
                    "Segments sentences into token-budget friendly JSONL batches for provider-agnostic translation.".into(),
                strategy: StageStrategy::Translate,
            },
            PipelineStage {
                name: "Validate placeholders".into(),
                description:
                    "Checks formatting tokens, placeholders, and markup parity before reinsertion.".into(),
                strategy: StageStrategy::Validate,
            },
            PipelineStage {
                name: "Repackage outputs".into(),
                description:
                    "Re-embeds .resources, updates .resx manifests, and rebuilds Harmony patches when applicable.".into(),
                strategy: StageStrategy::Repackage,
            },
        ];

        let validators = vec![
            ValidatorSpec {
                name: "Placeholder parity".into(),
                description: "Ensures numbered and printf-style placeholders match the source count and order.".into(),
            },
            ValidatorSpec {
                name: "Length guard".into(),
                description: "Flags strings that exceed configurable length limits or line counts.".into(),
            },
            ValidatorSpec {
                name: "Markup preservation".into(),
                description: "Verifies BBCode/HTML tags are preserved and properly nested.".into(),
            },
        ];

        let skip_rules = vec![
            "Unity AssetBundles and opaque binaries are skipped by default.".into(),
            "Managed DLLs prefer resource-first editing; Harmony patches are used for code hooks."
                .into(),
        ];

        Self {
            target,
            stages,
            validators,
            skip_rules,
        }
    }
}
