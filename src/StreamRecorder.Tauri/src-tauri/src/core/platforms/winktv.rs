use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    super::pandatv::probe_site(ctx, input, "winktv", "WinkTV", "media").await
}
