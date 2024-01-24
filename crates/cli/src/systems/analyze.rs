use starbase::system;

#[system]
pub async fn load_toolchain() {
    moon::load_toolchain().await?;
}
