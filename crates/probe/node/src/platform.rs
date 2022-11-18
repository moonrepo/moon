use probe_core::ProbeError;
use std::fmt;

pub enum NodeArch {
    Arm64,
    Armv7l,
    Ppc64,
    Ppc64le,
    S390x,
    X64,
    X86,
}

impl std::str::FromStr for NodeArch {
    type Err = ProbeError;

    fn from_str(s: &str) -> Result<NodeArch, Self::Err> {
        match s {
            "arm" => Ok(NodeArch::Armv7l),
            "arm64" => Ok(NodeArch::Arm64),
            "powerpc" => Ok(NodeArch::Ppc64le),
            "powerpc64" => Ok(NodeArch::Ppc64),
            "s390x" => Ok(NodeArch::S390x),
            "x86_64" => Ok(NodeArch::X64),
            "x86" => Ok(NodeArch::X86),
            unknown => Err(ProbeError::UnsupportedArchitecture(
                "Node.js".into(),
                unknown.to_owned(),
            )),
        }
    }
}

impl fmt::Display for NodeArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                NodeArch::Arm64 => "arm64",
                NodeArch::Armv7l => "armv7l",
                NodeArch::Ppc64 => "ppc64",
                NodeArch::Ppc64le => "ppc64le",
                NodeArch::S390x => "s390x",
                NodeArch::X64 => "x64",
                NodeArch::X86 => "x86",
            }
        )
    }
}
