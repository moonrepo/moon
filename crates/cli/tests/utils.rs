use moon_test_utils2::{MoonSandbox, create_moon_sandbox};

pub fn create_projects_sandbox() -> MoonSandbox {
    let sandbox = create_moon_sandbox("projects");
    sandbox.with_default_projects();
    sandbox
}
