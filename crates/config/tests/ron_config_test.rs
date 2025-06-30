#[cfg(feature = "loader")]
mod ron_tests {
    use moon_config::ConfigLoader;
    use std::path::Path;

    #[test]
    fn can_load_ron_config() {
        let loader = ConfigLoader::default();
        let ron_path = Path::new("tests/__fixtures__/ron");
        
        // Try to load a project config
        let result = loader.load_project_config(ron_path);
        
        assert!(result.is_ok(), "Failed to load RON config: {:?}", result.err());
        
        let config = result.unwrap();
        assert_eq!(config.id, Some("custom-id".to_string()));
        assert_eq!(config.type_of, Some("library".to_string()));
        assert_eq!(config.language, Some("rust".to_string()));
    }
}