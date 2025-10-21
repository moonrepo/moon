use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn register_extension(
    Json(input): Json<RegisterExtensionInput>,
) -> FnResult<Json<RegisterExtensionOutput>> {
    Ok(Json(RegisterExtensionOutput {
        name: input.id.to_string(),
        ..Default::default()
    }))
}
