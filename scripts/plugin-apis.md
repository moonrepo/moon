# WASM toolchain plugins

## Tier 1 - 8/8

- [x] `register_toolchain` \*
- [x] `initialize_toolchain`
- [x] `detect_version_files`
- [x] `parse_version_file`
- [x] `define_toolchain_config`
- [x] `define_docker_metadata`
- [x] `scaffold_docker`
- [x] `prune_docker`

## Tier 2 - 3/11

- [ ] `extend_project`
- [ ] `extend_task`
- [ ] `extend_task_command`
- [ ] `hash_manifest_contents`
- [x] `hash_task_contents`
- [x] `install_dependencies`
- [x] `locate_dependencies_root`
- [ ] `parse_lockfile`
- [ ] `parse_manifest`
- [x] `sync_workspace`
- [x] `sync_project`

## Tier 3 - 8/8

- [x] `register_tool` \*
- [x] `download_prebuilt` or `native_install` \*
- [x] `unpack_archive`
- [x] `locate_executables` \*
- [x] `load_versions` \*
- [x] `resolve_version` \*
- [x] `setup_toolchain`
- [x] `teardown_toolchain`
