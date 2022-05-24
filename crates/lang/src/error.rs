use thiserror::Error;

#[derive(Error, Debug)]
pub enum LangError {
    #[error(
        "Shashum check has failed for <file>{0}</file>, which was downloaded from <url>{1}</url>."
    )]
    InvalidShasum(
        String, // Download path
        String, // URL
    ),

    #[error(
        "Unsupported architecture <symbol>{0}</symbol>. Unable to install <symbol>{1}</symbol>."
    )]
    UnsupportedArchitecture(
        String, // Arch
        String, // Tool name
    ),

    #[error("Unsupported platform <symbol>{0}</symbol>. Unable to install <symbol>{1}</symbol>.")]
    UnsupportedPlatform(
        String, // Platform
        String, // Tool name
    ),
}
