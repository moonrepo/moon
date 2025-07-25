Update this list based on crates that have `publish = true`.

```
cargo release patch --execute -p moon_common -p moon_config -p moon_feature_flags -p moon_file_group -p moon_pdk -p moon_pdk_api -p moon_pdk_test_utils -p moon_project -p moon_target -p moon_task
```

```
cargo release patch --execute -p moon_common -p moon_config -p moon_feature_flags -p moon_file_group -p moon_project -p moon_target -p moon_task
cargo release minor --execute -p moon_pdk -p moon_pdk_api -p moon_pdk_test_utils
```
