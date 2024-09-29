use a0::generate;

#[generate("tests/struct.json")]
struct Json;

#[generate("tests/struct.toml")]
struct Toml;

#[generate("tests/struct.yaml")]
struct Yaml;

#[generate("tests/struct.conf", "json")]
struct Generic;

mod tests {
    use super::*;

    #[test]
    fn test_json_struct() {
        let json_struct = Json::new();
        assert_eq!(json_struct.data, "test", "JSON struct data should be 'test'");
        assert!(!json_struct.is_empty(), "JSON struct should not be empty");

        // Test default values
        assert_eq!(json_struct.optional_field, None, "Optional field should be None by default");
        assert_eq!(json_struct.number_field, 0, "Number field should default to 0");

        // Test debug output
        assert!(!format!("{:?}", json_struct).is_empty(), "Debug output should not be empty");
    }

    #[test]
    fn test_toml_struct() {
        let toml_struct = Toml::new();
        assert_eq!(toml_struct.data, "test", "TOML struct data should be 'test'");
        assert!(!toml_struct.is_empty(), "TOML struct should not be empty");

        // Test any TOML-specific fields
        assert!(toml_struct.toml_specific_field, "TOML-specific field should be present");

        // Test clone if implemented
        let cloned = toml_struct.clone();
        assert_eq!(toml_struct, cloned, "Cloned TOML struct should be equal to original");
    }

    #[test]
    fn test_yaml_struct() {
        let yaml_struct = Yaml::new();
        assert_eq!(yaml_struct.data, "test", "YAML struct data should be 'test'");
        assert!(!yaml_struct.is_empty(), "YAML struct should not be empty");

        // Test any YAML-specific fields
        assert!(yaml_struct.yaml_list.len() > 0, "YAML list should not be empty");
    }

    #[test]
    fn test_generic_struct() {
        let generic_struct = Generic::new();
        assert_eq!(generic_struct.data, "test", "Generic struct data should be 'test'");
        assert!(!generic_struct.is_empty(), "Generic struct should not be empty");

        // Test generic fields
        assert!(generic_struct.config_type == "json", "Config type should be 'json'");
    }
}
