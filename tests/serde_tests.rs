use serde::de::DeserializeSeed;
use yuna::serde::{ConfigFile, Mapping, ScalarType, ValueWithSource, config_file_expected_message};

#[test]
fn visitor_expected_message_is_exposed() {
  assert_eq!(config_file_expected_message(), "any YAML value");
}

#[test]
fn value_helpers_work() {
  let mut mapping = ValueWithSource::Mapping(Mapping::new());
  assert!(mapping.is_mapping());
  assert!(!mapping.is_sequence());
  assert!(mapping.as_mapping_mut().is_some());
  assert!(mapping.as_sequence_mut().is_none());

  mapping = ValueWithSource::Sequence(vec![]);
  assert!(mapping.is_sequence());
  assert!(!mapping.is_mapping());
  assert!(mapping.as_sequence_mut().is_some());
  assert!(mapping.as_mapping_mut().is_none());
}

#[test]
fn serialize_scalars_mappings_sequences_and_fallbacks() {
  let bool_ok = ValueWithSource::Scalar {
    value: "true".to_string(),
    source: "a.yml".into(),
    original_type: ScalarType::Boolean,
  };
  let bool_fallback = ValueWithSource::Scalar {
    value: "not-bool".to_string(),
    source: "a.yml".into(),
    original_type: ScalarType::Boolean,
  };
  let int_ok = ValueWithSource::Scalar {
    value: "42".to_string(),
    source: "a.yml".into(),
    original_type: ScalarType::Integer,
  };
  let int_fallback = ValueWithSource::Scalar {
    value: "42x".to_string(),
    source: "a.yml".into(),
    original_type: ScalarType::Integer,
  };
  let float_ok = ValueWithSource::Scalar {
    value: "1.5".to_string(),
    source: "a.yml".into(),
    original_type: ScalarType::Float,
  };
  let float_fallback = ValueWithSource::Scalar {
    value: "1.5x".to_string(),
    source: "a.yml".into(),
    original_type: ScalarType::Float,
  };
  let string_ok = ValueWithSource::Scalar {
    value: "hello".to_string(),
    source: "a.yml".into(),
    original_type: ScalarType::String,
  };

  assert_eq!(serde_yml::to_string(&bool_ok).expect("bool"), "true\n");
  assert!(
    serde_yml::to_string(&bool_fallback)
      .expect("bool fallback")
      .contains("not-bool")
  );
  assert_eq!(serde_yml::to_string(&int_ok).expect("int"), "42\n");
  assert!(
    serde_yml::to_string(&int_fallback)
      .expect("int fallback")
      .contains("42x")
  );
  assert_eq!(serde_yml::to_string(&float_ok).expect("float"), "1.5\n");
  assert!(
    serde_yml::to_string(&float_fallback)
      .expect("float fallback")
      .contains("1.5x")
  );
  assert_eq!(serde_yml::to_string(&string_ok).expect("string"), "hello\n");

  let null = ValueWithSource::Null;
  assert_eq!(serde_yml::to_string(&null).expect("null"), "null\n");

  let mut map = Mapping::new();
  map.insert("a".to_string(), int_ok.clone());
  let wrapped = ValueWithSource::Mapping(map);
  let serialized_map = serde_yml::to_string(&wrapped).expect("mapping");
  assert!(serialized_map.contains("a: 42"));

  let sequence = ValueWithSource::Sequence(vec![string_ok, bool_ok]);
  let serialized_sequence = serde_yml::to_string(&sequence).expect("sequence");
  assert!(serialized_sequence.contains("- hello"));
  assert!(serialized_sequence.contains("- true"));
}

#[test]
fn deserialize_seed_tracks_source_and_types() {
  let yaml = r#"
b: true
i: -2
u: 9223372036854775808
f: 1.5
s: hi
n: null
seq: [1, "a"]
map: {k: v}
"#;

  let de = serde_yml::Deserializer::from_str(yaml);
  let expected_source = std::path::PathBuf::from("source.yml");
  let root = ConfigFile {
    source: expected_source.clone(),
  }
  .deserialize(de)
  .expect("deserialize seeded config");

  let map = match root {
    ValueWithSource::Mapping(m) => m,
    _ => panic!("root must be a mapping"),
  };

  assert!(matches!(map.get("n"), Some(ValueWithSource::Null)));

  match map.get("b").expect("b") {
    ValueWithSource::Scalar {
      value,
      source,
      original_type,
    } => {
      assert_eq!(value, "true");
      assert_eq!(source, &expected_source);
      assert!(matches!(original_type, ScalarType::Boolean));
    }
    _ => panic!("b should be scalar"),
  }

  assert!(matches!(
    map.get("i"),
    Some(ValueWithSource::Scalar {
      original_type: ScalarType::Integer,
      ..
    })
  ));
  assert!(matches!(
    map.get("u"),
    Some(ValueWithSource::Scalar {
      original_type: ScalarType::Integer,
      ..
    })
  ));
  assert!(matches!(
    map.get("f"),
    Some(ValueWithSource::Scalar {
      original_type: ScalarType::Float,
      ..
    })
  ));
  assert!(matches!(
    map.get("s"),
    Some(ValueWithSource::Scalar {
      original_type: ScalarType::String,
      ..
    })
  ));
  assert!(matches!(map.get("seq"), Some(ValueWithSource::Sequence(_))));
  assert!(matches!(map.get("map"), Some(ValueWithSource::Mapping(_))));
}
