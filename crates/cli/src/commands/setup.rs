use moon_workspace::Workspace;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;
    let mut root_package = workspace.load_package_json().await?;

    workspace.toolchain.setup(&mut root_package).await?;

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use crate::helpers::create_test_command;

//     #[test]
//     fn installs() {
//         let assert = create_test_command("base")
//             .arg("--log-level")
//             .arg("trace")
//             .arg("setup")
//             .assert();

//         assert.success().code(0);
//     }
// }
