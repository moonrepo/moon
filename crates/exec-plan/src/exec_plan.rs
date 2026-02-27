use moon_affected::{DownstreamScope, UpstreamScope};
use moon_common::{cacheable, cacheable_enum};

cacheable!(
    #[derive(Default)]
    #[serde(default, deny_unknown_fields)]
    pub struct AffectedBlock {
        pub base: Option<String>,
        pub head: Option<String>,
        pub include_relations: Option<bool>,
        pub status: Vec<moon_vcs::ChangedStatus>,
        pub stdin: Option<bool>,
    }
);

cacheable!(
    #[derive(Default)]
    #[serde(default, deny_unknown_fields)]
    pub struct GraphBlock {
        pub downstream: Option<DownstreamScope>,
        pub upstream: Option<UpstreamScope>,
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
        pub on_failure: Option<OnFailure>,
        pub job: Option<usize>,
        pub job_total: Option<usize>,
    }
);

cacheable!(
    #[derive(Default)]
    #[serde(default, deny_unknown_fields)]
    pub struct ExecutionPlan {
        pub affected: Option<AffectedBlock>,
        pub graph: GraphBlock,
        pub pipeline: PipelineBlock,
    }
);
