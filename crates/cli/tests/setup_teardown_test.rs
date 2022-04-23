// NOTE: teardown removes the ~/.moon dir entirely, disrupting other tests.
// Need to revisit and isolate this somehow...

// use moon_utils::path::get_home_dir;
// use moon_utils::test::create_moon_command;

// // We use a different Node.js version as to not conflict with other tests!
// #[test]
// fn sets_up_and_tears_down() {
//     let node_version = "16.1.0";
//     let home_dir = get_home_dir().unwrap();
//     let moon_dir = home_dir.join(".moon");
//     let node_dir = moon_dir.join("tools").join("node").join(node_version);

//     assert!(!node_dir.exists());

//     let setup = create_moon_command("cases")
//         .arg("--logLevel")
//         .arg("trace")
//         .arg("setup")
//         .env("MOON_NODE_VERSION", node_version)
//         .assert();

//     setup.success().code(0);

//     assert!(node_dir.exists());

//     let teardown = create_moon_command("cases")
//         .arg("--logLevel")
//         .arg("trace")
//         .arg("teardown")
//         .env("MOON_NODE_VERSION", node_version)
//         .assert();

//     teardown.success().code(0);

//     assert!(!node_dir.exists());
// }
