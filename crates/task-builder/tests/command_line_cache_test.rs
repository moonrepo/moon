use moon_common::Id;
use moon_config::TaskArgs;
use moon_target::Target;
use moon_task::{Task, TaskArg};
use moon_task_builder::{CommandLineCache, parse_command_line};
use moon_test_utils::WorkspaceMocker;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeMap;
use std::path::Path;

fn create_mocker(root: &Path) -> WorkspaceMocker {
    WorkspaceMocker::new(root)
        .with_default_projects()
        .with_global_envs()
        .load_inherited_tasks_from("global")
}

async fn build_tasks(mocker: &WorkspaceMocker, id: &str) -> BTreeMap<Id, Task> {
    let project = mocker.build_project(id).await;
    mocker.build_tasks(&project).await
}

fn target(id: &str) -> Target {
    Target::parse(id).unwrap()
}

fn string(cmd: &str) -> TaskArgs {
    TaskArgs::String(cmd.into())
}

const CORPUS: &[&str] = &[
    "bin",
    "bin --foo bar",
    "foo -a --bar baz qux",
    "foo -- bar -b",
    "foo --",
    "echo noquotes 'single quote' \"double quote\" $\"special quote\"",
    "KEY=value OTHER=\"quoted\" bin --flag",
    "KEY=$VAR bin",
    "KEY=value; bin arg",
    "bin $HOME ${VAR}",
    "bin $(echo sub) `echo tick`",
    "bin ~/path {a,b}",
    "bin @files(sources) --opt=value",
    "bin %(murex brace) r#'nu raw'#",
    "",
];

mod command_line_cache {
    use super::*;

    #[test]
    fn hit_returns_same_result_as_cold_parse() {
        let cache = CommandLineCache::default();

        for cmd in CORPUS {
            let cold = parse_command_line(&target("a:task"), &string(cmd)).unwrap();
            let miss = cache.parse(&target("a:task"), &string(cmd)).unwrap();
            let hit = cache.parse(&target("a:task"), &string(cmd)).unwrap();

            assert_eq!(cold, miss, "miss differs for {cmd:?}");
            assert_eq!(cold, hit, "hit differs for {cmd:?}");
        }

        assert_eq!(cache.len(), CORPUS.len());
    }

    #[test]
    fn hit_preserves_env_and_shell_detection() {
        let cache = CommandLineCache::default();

        cache
            .parse(&target("a:task"), &string("KEY=value bin $(echo sub)"))
            .unwrap();

        let hit = cache
            .parse(&target("a:task"), &string("KEY=value bin $(echo sub)"))
            .unwrap();

        assert_eq!(
            hit.args.unwrap(),
            [
                TaskArg::new_unquoted("bin"),
                TaskArg::new_unquoted("$(echo sub)")
            ]
        );
        assert_eq!(
            hit.env.unwrap().get("KEY").cloned().flatten().as_deref(),
            Some("value")
        );
        assert!(hit.requires_shell);
    }

    #[test]
    fn result_is_shared_across_targets() {
        let cache = CommandLineCache::default();

        let a = cache
            .parse(&target("a:build"), &string("bin --foo"))
            .unwrap();
        let b = cache
            .parse(&target("b:build"), &string("bin --foo"))
            .unwrap();
        let c = cache
            .parse(&target("c:other"), &string("bin --foo"))
            .unwrap();

        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn different_strings_do_not_collide() {
        let cache = CommandLineCache::default();

        let a = cache
            .parse(&target("a:task"), &string("bin --foo"))
            .unwrap();
        let b = cache
            .parse(&target("a:task"), &string("bin --bar"))
            .unwrap();
        let c = cache
            .parse(&target("a:task"), &string("bin  --foo"))
            .unwrap();

        assert_ne!(a, b);
        assert_eq!(cache.len(), 3);

        // Same args, but a different key, as the raw string is the key
        assert_eq!(a, c);

        assert_eq!(
            cache
                .parse(&target("a:task"), &string("bin --foo"))
                .unwrap()
                .args
                .unwrap(),
            ["bin", "--foo"]
        );
        assert_eq!(
            cache
                .parse(&target("a:task"), &string("bin --bar"))
                .unwrap()
                .args
                .unwrap(),
            ["bin", "--bar"]
        );
    }

    #[test]
    fn does_not_cache_lists_or_noop() {
        let cache = CommandLineCache::default();

        let list = TaskArgs::List(vec!["bin".into(), "--foo".into()]);

        assert_eq!(
            cache.parse(&target("a:task"), &list).unwrap(),
            parse_command_line(&target("a:task"), &list).unwrap()
        );
        assert_eq!(
            cache.parse(&target("a:task"), &TaskArgs::Noop).unwrap(),
            parse_command_line(&target("a:task"), &TaskArgs::Noop).unwrap()
        );
        assert!(cache.is_empty());
    }

    #[test]
    fn does_not_cache_failures() {
        let cache = CommandLineCache::default();

        for cmd in [
            "echo foo | grep f",
            "echo foo && echo bar",
            "echo foo > file.txt",
        ] {
            let a = cache.parse(&target("a:task"), &string(cmd)).unwrap_err();
            let b = cache.parse(&target("b:other"), &string(cmd)).unwrap_err();

            // Errors embed the target, so each must report its own
            assert!(a.to_string().contains("a:task"), "{a}");
            assert!(b.to_string().contains("b:other"), "{b}");
        }

        assert!(cache.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn builds_identical_tasks_with_a_warm_cache() {
        let sandbox = create_sandbox("builder");

        let cold = create_mocker(sandbox.path());
        let cold_tasks = build_tasks(&cold, "commands").await;

        // Inherited global tasks share the same strings
        assert!(!cold.command_line_cache.is_empty());

        // Warm the cache with other projects first
        let warm = create_mocker(sandbox.path());
        build_tasks(&warm, "local").await;
        build_tasks(&warm, "syntax").await;
        build_tasks(&warm, "commands").await;

        let warm_tasks = build_tasks(&warm, "commands").await;

        assert_eq!(cold_tasks, warm_tasks);
    }
}
