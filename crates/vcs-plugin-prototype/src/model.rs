//! PROTOTYPE: Pure composition logic, intentionally independent of Extism.

use moon_pdk_api::VcsStatePatch;
use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
pub struct AdapterState {
    pub adapter: String,
    pub current_label: String,
    pub current_revision: String,
    pub default_label: String,
    pub default_revision: String,
    pub is_default: bool,
    pub repository_root: String,
    pub working_root: String,
}

pub fn compose_state(base: &AdapterState, patch: Option<&VcsStatePatch>) -> AdapterState {
    let Some(patch) = patch else {
        return base.clone();
    };

    AdapterState {
        adapter: patch
            .adapter
            .clone()
            .unwrap_or_else(|| base.adapter.clone()),
        current_label: patch
            .current_label
            .clone()
            .unwrap_or_else(|| base.current_label.clone()),
        current_revision: patch
            .current_revision
            .clone()
            .unwrap_or_else(|| base.current_revision.clone()),
        is_default: patch.is_default.unwrap_or(base.is_default),
        repository_root: patch
            .repository_root
            .clone()
            .unwrap_or_else(|| base.repository_root.clone()),
        working_root: patch
            .working_root
            .clone()
            .unwrap_or_else(|| base.working_root.clone()),
        default_label: base.default_label.clone(),
        default_revision: base.default_revision.clone(),
    }
}
