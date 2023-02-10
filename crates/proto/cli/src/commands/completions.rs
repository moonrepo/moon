use crate::app::App;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use proto::ProtoError;

pub async fn completions(shell: Option<Shell>) -> Result<(), ProtoError> {
    let Some(shell) = shell.or_else(Shell::from_env) else {
      return Err(ProtoError::UnsupportedShell);
    };

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    generate(shell, &mut app, "proto", &mut stdio);

    // let root_dir = get_root()?;
    // let home_dir = root_dir.parent().unwrap();
    // let file_path = match shell {
    //     Shell::Bash => home_dir.join(".bash_completion.d/proto.sh"),
    //     Shell::Elvish => todo!(),
    //     Shell::Fish => home_dir.join(".config/fish/completions/proto.fish"),
    //     Shell::PowerShell => todo!(),

    //     // These shells dont hae a standard location to write completion files,
    //     // so we output to stdout instead!
    //     _ => {
    //         let mut stdio = std::io::stdout();

    //         shell.generate(&app, &mut stdio);

    //         return Ok(());
    //     }
    // };

    // trace!(
    //     target: "proto:completions",
    //     "Writing completions to {}",
    //     color::path(&file_path),
    // );

    // let mut file = std::fs::File::open(&file_path)
    //     .map_err(|e| ProtoError::Fs(file_path.to_path_buf(), e.to_string()))?;

    // shell.generate(&app, &mut file);

    // info!(
    //     target: "proto:completions",
    //     "Generated completions for {} shell",
    //     shell
    // );

    Ok(())
}
