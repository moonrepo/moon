use moon_app_components::Console;
use starbase::system;

#[system]
pub async fn flush_outputs(console: ResourceMut<Console>) {
    console.close()?;
}
