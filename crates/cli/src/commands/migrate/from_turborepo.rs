use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn from_turborepo() {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon ext migrate-turborepo")
    );
}
