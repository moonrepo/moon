use moon_config::ProjectID;

#[derive(Debug)]
pub struct GraphEdge {
    pub source: ProjectID,
    pub target: ProjectID,
}
