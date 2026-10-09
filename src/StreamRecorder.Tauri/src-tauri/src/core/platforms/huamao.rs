use crate::core::probe::*;
pub(super) async fn probe(ctx: &ProbeContext, input: &ProbeInput) -> Result<StreamData, String> {
    let mut data = super::piaopiao::web(ctx, input).await?;
    data.platform = Some("花猫直播".into());
    Ok(data)
}
