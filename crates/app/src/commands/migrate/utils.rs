use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_config_loader::{read_config_based_on_extension, write_config_based_on_extension};
use moon_console::ConsoleExt;
use moon_console::ui::Confirm;
use starbase_utils::yaml::{self, YamlMapping, YamlValue};
use starbase_utils::{glob, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, warn};

// PROMPTS

/// Confirm that configuration files should be migrated, unless prompts are skipped.
pub async fn confirm_config_migration(
    session: &MoonSession,
    skip_prompts: bool,
) -> miette::Result<bool> {
    if skip_prompts {
        return Ok(true);
    }

    let mut confirmed = false;

    session
        .console
        .render_prompt(element! {
            Confirm(
                label: "Migrate configuration files?".to_string(),
                description: "This will strip comments and re-format the files that are changed.".to_string(),
                on_confirm: &mut confirmed
            )
        })
        .await?;

    Ok(confirmed)
}

// SETTINGS

fn split_key(key: &str) -> (&str, &str) {
    key.rsplit_once('.').unwrap_or(("", key))
}

/// Run the operation on the mapping that contains the last part of the
/// dot-separated key, but only if every parent mapping exists.
pub fn with_setting_parent(
    root: &mut YamlMapping,
    key: &str,
    op: impl FnOnce(&mut YamlMapping, &str),
) {
    let (parent_key, name) = split_key(key);
    let mut node = root;

    for part in parent_key.split('.').filter(|part| !part.is_empty()) {
        match node.get_mut(part).and_then(YamlValue::as_mapping_mut) {
            Some(next) => node = next,
            None => return,
        };
    }

    op(node, name);
}

fn create_setting_parent<'a>(
    root: &'a mut YamlMapping,
    parent_key: &str,
) -> Option<&'a mut YamlMapping> {
    let mut node = root;

    for part in parent_key.split('.').filter(|part| !part.is_empty()) {
        node = node
            .entry(YamlValue::from(part))
            .or_insert_with(|| YamlValue::Mapping(YamlMapping::new()))
            .as_mapping_mut()?;
    }

    Some(node)
}

pub fn has_setting(root: &mut YamlMapping, key: &str) -> bool {
    let mut exists = false;

    with_setting_parent(root, key, |node, name| {
        exists = node.contains_key(name);
    });

    exists
}

/// Set a dot-separated setting, and create its parent mappings when missing.
/// An existing setting keeps its position.
pub fn set_setting(root: &mut YamlMapping, key: &str, value: YamlValue) {
    let (parent_key, name) = split_key(key);

    if let Some(node) = create_setting_parent(root, parent_key) {
        node.insert(YamlValue::from(name), value);
    }
}

pub fn remove_setting(root: &mut YamlMapping, key: &str) -> Option<YamlValue> {
    let mut value = None;

    // Shift instead of swap, so that the order of keys is preserved
    with_setting_parent(root, key, |node, name| {
        value = node.shift_remove(name);
    });

    value
}

pub fn remove_setting_if_empty(root: &mut YamlMapping, key: &str) {
    with_setting_parent(root, key, |node, name| {
        if node
            .get(name)
            .and_then(YamlValue::as_mapping)
            .is_some_and(|map| map.is_empty())
        {
            node.shift_remove(name);
        }
    });
}

pub fn rename_key(node: &mut YamlMapping, old_name: &str, new_name: &str) {
    if !node.contains_key(old_name) {
        return;
    }

    // Rebuild the mapping, so that the renamed key keeps its position
    *node = std::mem::take(node)
        .into_iter()
        .map(|(key, value)| {
            if key.as_str() == Some(old_name) {
                (YamlValue::from(new_name), value)
            } else {
                (key, value)
            }
        })
        .collect();
}

/// Rename (or move) a dot-separated setting. If the new setting has
/// already been defined, it takes precedence and the old one is removed.
pub fn rename_setting(root: &mut YamlMapping, old_key: &str, new_key: &str) {
    if has_setting(root, new_key) {
        remove_setting(root, old_key);
        return;
    }

    let (old_parent_key, old_name) = split_key(old_key);
    let (new_parent_key, new_name) = split_key(new_key);

    if old_parent_key == new_parent_key {
        with_setting_parent(root, old_key, |node, _| {
            rename_key(node, old_name, new_name);
        });

        return;
    }

    if let Some(value) = remove_setting(root, old_key) {
        set_setting(root, new_key, value);
    }
}

// FILES

pub fn find_project_config_files(
    session: &MoonSession,
    extensions: &[&str],
) -> miette::Result<Vec<PathBuf>> {
    let patterns = [
        format!("**/moon.{{{}}}", extensions.join(",")),
        "!**/node_modules/**".to_owned(),
    ];

    Ok(glob::walk_files(&session.workspace_root, &patterns)?)
}

pub fn warn_pkl_config_files() {
    static PKL_WARNED: AtomicBool = AtomicBool::new(false);

    if !PKL_WARNED.swap(true, Ordering::Relaxed) {
        warn!("Pkl based configuration files cannot be automatically migrated!");
    }
}

/// Read a configuration file of any format, except Pkl, into a YAML value.
/// When it cannot be read, a warning is logged and `None` is returned.
pub fn read_config_file(config_path: &Path) -> Option<YamlValue> {
    if config_path.extension().is_some_and(|ext| ext == "pkl") {
        warn_pkl_config_files();

        return None;
    }

    match read_config_based_on_extension(config_path) {
        Ok(config) => Some(config),
        Err(error) => {
            warn!(
                config = ?config_path,
                "Failed to parse configuration file, skipping migration: {error}"
            );

            None
        }
    }
}

pub fn write_config_file(config_path: &Path, config: &YamlValue) -> miette::Result<()> {
    // Respect the closest `.editorconfig` when possible
    match config_path.extension().and_then(|ext| ext.to_str()) {
        Some("json" | "jsonc") => json::write_file_with_config(config_path, config, true)?,
        Some("yml" | "yaml") => yaml::write_file_with_config(config_path, config)?,
        _ => write_config_based_on_extension(config_path, config)?,
    };

    Ok(())
}

/// Migrate the settings of a configuration file, and only write the
/// file when a setting has changed, so that comments are preserved
/// in files that do not require a migration.
pub fn migrate_config_file(
    config_path: &Path,
    op: impl FnOnce(&mut YamlMapping, &Path),
) -> miette::Result<()> {
    let Some(mut config) = read_config_file(config_path) else {
        return Ok(());
    };

    let Some(root) = config.as_mapping_mut() else {
        return Ok(());
    };

    let original = root.clone();

    op(root, config_path);

    if *root == original {
        return Ok(());
    }

    debug!(config = ?config_path, "Migrating configuration file");

    write_config_file(config_path, &config)
}

#[cfg(test)]
mod test {
    use super::*;

    fn create_root(code: &str) -> YamlMapping {
        yaml::parse(code).unwrap()
    }

    fn format_root(root: &YamlMapping) -> String {
        yaml::format(root).unwrap()
    }

    mod rename_setting {
        use super::*;

        #[test]
        fn renames_in_place() {
            let mut root = create_root("a: 1\nb: 2\nc: 3\n");

            rename_setting(&mut root, "b", "d");

            assert_eq!(format_root(&root), "a: 1\nd: 2\nc: 3\n");
        }

        #[test]
        fn renames_nested_in_place() {
            let mut root = create_root("a:\n  b: 1\n  c: 2\n");

            rename_setting(&mut root, "a.b", "a.d");

            assert_eq!(format_root(&root), "a:\n  d: 1\n  c: 2\n");
        }

        #[test]
        fn moves_to_a_new_parent() {
            let mut root = create_root("a:\n  b: 1\n  c: 2\n");

            rename_setting(&mut root, "a.b", "d.e");

            assert_eq!(format_root(&root), "a:\n  c: 2\nd:\n  e: 1\n");
        }

        #[test]
        fn keeps_the_new_setting() {
            let mut root = create_root("a: 1\nb: 2\n");

            rename_setting(&mut root, "a", "b");

            assert_eq!(format_root(&root), "b: 2\n");
        }

        #[test]
        fn does_nothing_when_missing() {
            let mut root = create_root("a: 1\n");

            rename_setting(&mut root, "b.c", "d.e");

            assert_eq!(format_root(&root), "a: 1\n");
        }
    }

    mod remove_setting {
        use super::*;

        #[test]
        fn preserves_order() {
            let mut root = create_root("a: 1\nb: 2\nc: 3\n");

            assert_eq!(remove_setting(&mut root, "a"), Some(YamlValue::from(1)));
            assert_eq!(format_root(&root), "b: 2\nc: 3\n");
        }

        #[test]
        fn removes_empty_mappings_only() {
            let mut root = create_root("a: {}\nb:\n  c: 1\n");

            remove_setting_if_empty(&mut root, "a");
            remove_setting_if_empty(&mut root, "b");

            assert_eq!(format_root(&root), "b:\n  c: 1\n");
        }
    }

    mod set_setting {
        use super::*;

        #[test]
        fn creates_parents() {
            let mut root = create_root("a: 1\n");

            set_setting(&mut root, "b.c", YamlValue::from(2));

            assert_eq!(format_root(&root), "a: 1\nb:\n  c: 2\n");
        }

        #[test]
        fn keeps_position() {
            let mut root = create_root("a: 1\nb: 2\n");

            set_setting(&mut root, "a", YamlValue::from(3));

            assert_eq!(format_root(&root), "a: 3\nb: 2\n");
        }
    }
}
