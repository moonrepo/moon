use moon_app_components::AppConsole;
use starbase::system;

#[system]
pub async fn flush_outputs(console: ResourceMut<AppConsole>) {
    console.close()?;
}
