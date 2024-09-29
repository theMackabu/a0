use a0::generate;

#[generate("tests/struct.json")]
struct Json;

#[generate("tests/struct.toml")]
struct Toml;

#[generate("tests/struct.yaml")]
struct Yaml;

#[generate("tests/struct.conf", "json")]
struct Generic;

#[test]
fn test_json_struct() {
    let json_struct = Json::new();

    assert_eq!(json_struct.data, "test");
    assert!(!json_struct.is_empty(), "JSON struct should not be empty");

    println!("JSON struct: {:?}", json_struct);
}

#[test]
fn test_toml_struct() {
    let toml_struct = Toml::new();

    assert_eq!(toml_struct.data, "test");
    assert!(!toml_struct.is_empty(), "TOML struct should not be empty");

    println!("TOML struct: {:?}", toml_struct);
}

#[test]
fn test_yaml_struct() {
    let yaml_struct = Yaml::new();

    assert_eq!(yaml_struct.data, "test");
    assert!(!yaml_struct.is_empty(), "YAML struct should not be empty");

    println!("YAML struct: {:?}", yaml_struct);
}

#[test]
fn test_generic_struct() {
    let generic_struct = Generic::new();

    assert_eq!(generic_struct.data, "test");
    assert!(!generic_struct.is_empty(), "Generic struct should not be empty");

    println!("Generic struct: {:?}", generic_struct);
}
