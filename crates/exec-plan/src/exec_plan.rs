use moon_affected::Depth;
use moon_common::{cacheable, cacheable_enum};
use moon_target::TargetLocator;

cacheable!(
    #[derive(Default)]
    #[serde(default, deny_unknown_fields)]
    pub struct AffectedBlock {
        pub base: Option<String>,
        pub head: Option<String>,

        /// How many hops away from an affected task its dependents are also
        /// affected: `none`, `direct`, `deep`, or a number.
        pub depth: Option<Depth>,

        pub source: Option<String>,
        pub status: Vec<moon_vcs::ChangedStatus>,
        pub stdin: Option<bool>,
    }
);

cacheable!(
    #[derive(Default)]
    #[serde(default, deny_unknown_fields)]
    pub struct GraphBlock {
        /// How many hops of dependents of the targets are also run:
        /// `none`, `direct`, `deep`, or a number.
        pub dependents: Option<Depth>,

        /// Don't run the dependencies of the targets that were not
        /// selected themselves.
        pub no_dependencies: Option<bool>,
    }
);

cacheable_enum!(
    pub enum OnFailure {
        Bail,
        Continue,
    }
);

cacheable!(
    #[derive(Default)]
    #[serde(default, deny_unknown_fields)]
    pub struct PipelineBlock {
        pub ci: Option<bool>,
        pub concurrency: Option<u8>,
        pub ignore_ci_checks: Option<bool>,
        pub no_actions: Option<bool>,
        pub on_failure: Option<OnFailure>,
        pub job: Option<usize>,
        pub job_total: Option<usize>,
    }
);

cacheable_enum!(
    #[derive(Clone, Debug)]
    #[serde(untagged)]
    pub enum TargetsBlock {
        Partitioned { jobs: Vec<Vec<TargetLocator>> },

        Filtered { include: Vec<TargetLocator> },

        Included(Vec<TargetLocator>),
    }
);

impl Default for TargetsBlock {
    fn default() -> Self {
        Self::Included(vec![])
    }
}

impl TargetsBlock {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Partitioned { jobs } => jobs.is_empty(),
            Self::Filtered { include } => include.is_empty(),
            Self::Included(targets) => targets.is_empty(),
        }
    }
}

cacheable!(
    #[derive(Default)]
    #[serde(default, deny_unknown_fields)]
    pub struct ExecutionPlan {
        pub affected: Option<AffectedBlock>,
        pub graph: GraphBlock,
        pub pipeline: PipelineBlock,
        pub targets: TargetsBlock,
    }
);
