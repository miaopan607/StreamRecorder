use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    super::qiandurebo::probe_with_platform(ctx, input, "心动热播直播").await
}
