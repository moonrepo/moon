use moon_workspace::Workspace;

pub async fn teardown() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    workspace.toolchain.teardown().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::helpers::create_test_command;

    #[test]
    fn uninstalls() {
        let assert = create_test_command("base").arg("teardown").assert();

        assert.success().code(0);
    }
}
