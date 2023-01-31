use proto_core::ProtoError;
use std::env::consts;
use std::fmt;

// Not everything is supported at the moment...
pub enum GoArch {
    Amd64,  // x86_64
    Arm64,  // Arm64
    Armv6l, // Arm V6
    I386,
    S390x,
}

impl GoArch {
    // https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
    pub fn from_os_arch() -> Result<GoArch, ProtoError> {
        // from rust archs
        match consts::ARCH {
            "arm" => Ok(GoArch::Armv6l),
            "aarch64" => Ok(GoArch::Arm64),
            "s390x" => Ok(GoArch::S390x),
            "x86_64" => Ok(GoArch::Amd64),
            "x86" => Ok(GoArch::I386),
            unknown => Err(ProtoError::UnsupportedArchitecture(
                "Go".into(),
                unknown.to_owned(),
            )),
        }
    }
}

impl fmt::Display for GoArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GoArch::Amd64 => "amd64",
                GoArch::Arm64 => "arm64",
                GoArch::Armv6l => "armv6l",
                GoArch::S390x => "s390x",
                GoArch::I386 => "386",
            }
        )
    }
}
