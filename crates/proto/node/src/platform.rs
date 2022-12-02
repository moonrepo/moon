use proto_core::ProtoError;
use std::env::consts;
use std::fmt;

// Not everything is supported at the moment...
// https://nodejs.org/api/process.html#processarch
pub enum NodeArch {
    Arm,
    Arm64,
    Ppc64,
    S390x,
    X64,
    X86,
}

impl NodeArch {
    // https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
    pub fn from_os_arch() -> Result<NodeArch, ProtoError> {
        // from rust archs
        match consts::ARCH {
            "arm" => Ok(NodeArch::Arm),
            "aarch64" => Ok(NodeArch::Arm64),
            "powerpc64" => Ok(NodeArch::Ppc64),
            "s390x" => Ok(NodeArch::S390x),
            "x86_64" => Ok(NodeArch::X64),
            "x86" => Ok(NodeArch::X86),
            unknown => Err(ProtoError::UnsupportedArchitecture(
                "Node.js".into(),
                unknown.to_owned(),
            )),
        }
    }
}

impl fmt::Display for NodeArch {
    // https://nodejs.org/dist/v19.2.0/
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            // to node arch file names
            match self {
                NodeArch::Arm => "armv7l",
                NodeArch::Arm64 => "arm64",
                NodeArch::Ppc64 =>
                    if consts::OS == "linux" {
                        "ppc64le"
                    } else {
                        "ppc64"
                    },
                NodeArch::S390x => "s390x",
                NodeArch::X64 => "x64",
                NodeArch::X86 => "x86",
            }
        )
    }
}
