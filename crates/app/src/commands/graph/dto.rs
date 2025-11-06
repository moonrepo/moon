use moon_common::cacheable;

cacheable!(
    #[derive(Debug, Hash, Eq, PartialEq)]
    pub struct GraphNodeDto {
        pub id: usize,
        pub label: String,
    }
);

cacheable!(
    #[derive(Debug)]
    pub struct GraphEdgeDto {
        pub id: String,
        pub label: String,
        pub source: usize,
        pub target: usize,
    }
);

cacheable!(
    #[derive(Debug)]
    pub struct GraphInfoDto {
        pub nodes: Vec<GraphNodeDto>,
        pub edges: Vec<GraphEdgeDto>,
    }
);
