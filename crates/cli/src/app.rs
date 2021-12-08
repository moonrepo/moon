use clap::{crate_version, App, AppSettings, Arg, SubCommand};

pub fn create_app<'a, 'b>() -> App<'a, 'b> {
    App::new("Monolith")
        .bin_name("mono")
        .version(crate_version!())
        .about("First-class monorepo management.")
        .help_short("h")
        .version_short("v")
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::GlobalVersion)
        // bin
        .subcommand(
            SubCommand::with_name("bin")
                .about("Return an absolute path to a toolchain binary.")
                .arg(
                    Arg::with_name("tool")
                        .help("The tool to query.")
                        .index(1)
                        .required(true)
                        .possible_values(&["node", "npm", "npx", "pnpm", "yarn"]),
                ),
        )
        // run
        .subcommand(
            SubCommand::with_name("run")
                .about("Run a task within a project.")
                .arg(
                    Arg::with_name("target")
                        .help("The task target to run.")
                        .index(1)
                        .required(true),
                ),
        )
}
