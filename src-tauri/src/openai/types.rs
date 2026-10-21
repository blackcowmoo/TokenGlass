use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub(crate) name: String,
    pub(crate) tokens: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiUsage {
    pub(crate) total_billed: f64,
    pub(crate) today_usage: f64,
    pub(crate) currency: String,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) models: Vec<ModelUsage>,
    pub(crate) period_start: i64,
    pub(crate) period_end: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiUsageSnapshot {
    pub(crate) usage: OpenAiUsage,
    pub(crate) fetched_at: i64,
    pub(crate) source: String,
    pub(crate) stale: bool,
    pub(crate) refresh_error: Option<String>,
}
