use moon_app_components::{StderrConsole, StdoutConsole};
use starbase::system;

#[system]
pub async fn flush_outputs(resources: ResourcesMut) {
    {
        resources.get_mut::<StderrConsole>().flush()?;
    }

    {
        resources.get_mut::<StdoutConsole>().flush()?;
    }
}
