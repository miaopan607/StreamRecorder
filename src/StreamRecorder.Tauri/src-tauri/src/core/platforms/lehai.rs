use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    super::haixiu::probe_site(ctx, input, "lehaitv", "乐嗨直播").await
}
