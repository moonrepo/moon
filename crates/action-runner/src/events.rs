use moon_workspace::Workspace;

type OutputHash = String;

pub enum Event<'a> {
    TargetOutputArchive,
    TargetOutputHydrate,
    TargetOutputCheckCache(&'a Workspace, &'a OutputHash),
}
