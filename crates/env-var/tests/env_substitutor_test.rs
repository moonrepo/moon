use moon_env_var::*;
use rustc_hash::FxHashMap;

mod env_substitutor {
    use super::*;

    #[test]
    fn handles_flags_when_missing() {
        let mut sub = EnvSubstitutor::default();

        assert_eq!(sub.substitute("$KEY"), "$KEY");
        assert_eq!(sub.substitute("${KEY}"), "${KEY}");

        assert_eq!(sub.substitute("$KEY!"), "$KEY");
        assert_eq!(sub.substitute("${KEY!}"), "$KEY");

        assert_eq!(sub.substitute("$KEY?"), "");
        assert_eq!(sub.substitute("${KEY?}"), "");
    }

    #[test]
    fn handles_flags_when_not_missing() {
        let mut envs = FxHashMap::default();
        envs.insert("KEY".to_owned(), "value".to_owned());
        let mut sub = EnvSubstitutor::default().with_local_vars(&envs);

        assert_eq!(sub.substitute("$KEY"), "value");
        assert_eq!(sub.substitute("${KEY}"), "value");

        assert_eq!(sub.substitute("$KEY!"), "$KEY");
        assert_eq!(sub.substitute("${KEY!}"), "$KEY");

        assert_eq!(sub.substitute("$KEY?"), "value");
        assert_eq!(sub.substitute("${KEY?}"), "value");
    }

    #[test]
    fn returns_from_local_or_global_bags() {
        let global = GlobalEnvBag::default();
        global.set("SOURCE", "global");
        global.set("GLOBAL", "global");

        let mut local = FxHashMap::default();
        local.insert("SOURCE".to_owned(), "local".to_owned());
        local.insert("LOCAL".to_owned(), "local".to_owned());

        let mut sub = EnvSubstitutor::default()
            .with_local_vars(&local)
            .with_global_vars(&global);

        assert_eq!(sub.substitute("$GLOBAL"), "global");
        assert_eq!(sub.substitute("$LOCAL"), "local");
        assert_eq!(sub.substitute("$SOURCE"), "local");

        // Remove from local
        drop(sub);

        local.remove("SOURCE");

        let mut sub = EnvSubstitutor::default()
            .with_local_vars(&local)
            .with_global_vars(&global);

        assert_eq!(sub.substitute("$SOURCE"), "global");

        // Then remove from global
        global.remove("SOURCE");

        assert_eq!(sub.substitute("$SOURCE"), "$SOURCE");
    }
}
