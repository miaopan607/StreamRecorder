use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    super::changliao::probe_site(
        ctx,
        input,
        "ybw1666.com",
        "音播直播",
        "https://live.ybw1666.com/800005143?promoters=0",
    )
    .await
}
