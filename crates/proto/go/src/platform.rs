use proto_core::ProtoError;
use std::env::consts;
use std::fmt;

// Not everything is supported at the moment...
pub enum GoArch {
    Arm,
    Arm64,
    S390x,
    X64,
    X86,
}

impl GoArch {
    // https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
    pub fn from_os_arch() -> Result<GoArch, ProtoError> {
        // from rust archs
        match consts::ARCH {
            "arm" => Ok(GoArch::Arm),
            "aarch64" => Ok(GoArch::Arm64),
            "s390x" => Ok(GoArch::S390x),
            "x86_64" => Ok(GoArch::X64),
            "x86" => Ok(GoArch::X86),
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
            // to node arch file names
            match self {
                GoArch::Arm => "armv7l",
                GoArch::Arm64 => "arm64",
                GoArch::S390x => "s390x",
                GoArch::X64 => "x64",
                GoArch::X86 => "x86",
            }
        )
    }
}
