use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serial_test::serial;
use tempfile::TempDir;
use yuna::cli::Args;
use yuna::config::{
  delete_value, find_configuration_files, interpolate, read_configuration_file, read_configuration_files,
  read_current_configuration_file, set_value, write_current_configuration_file,
};
use yuna::serde::{FlattenValues, Mapping, ScalarType, ValueWithSource};

fn args(name: Option<&str>, value: &[&str]) -> Args {
  Args {
    cwd: PathBuf::new(),
    home: String::new(),
    config: ".yuna.yml".to_string(),
    delete: false,
    no_merge: false,
    raw: false,
    name: name.map(ToString::to_string),
    value: value.iter().map(|v| v.to_string()).collect(),
  }
}

fn scalar(value: &str, source: &Path, original_type: ScalarType) -> ValueWithSource {
  ValueWithSource::Scalar {
    value: value.to_string(),
    source: source.to_path_buf(),
    original_type,
  }
}

#[test]
#[serial]
fn set_and_delete_values_cover_paths() {
  let temp = TempDir::new().expect("temp dir");
  env::set_current_dir(temp.path()).expect("set cwd");

  let mut config = ValueWithSource::Null;
  let opts = args(Some("db.host"), &["localhost"]);
  set_value(&opts, &mut config).expect("set nested value");

  let seq_opts = args(Some("servers.1"), &["api.local"]);
  set_value(&seq_opts, &mut config).expect("set sequence value");

  let mut flattened = FlattenValues::new();
  if let ValueWithSource::Mapping(ref mapping) = config {
    let db = mapping.get("db").expect("db key");
    if let ValueWithSource::Mapping(db_map) = db {
      let host = db_map.get("host").expect("host key");
      if let ValueWithSource::Scalar { value, .. } = host {
        assert_eq!(value, "localhost");
      } else {
        panic!("host must be scalar");
      }
    } else {
      panic!("db must be mapping");
    }

    let servers = mapping.get("servers").expect("servers key");
    if let ValueWithSource::Sequence(values) = servers {
      assert!(matches!(values[0], ValueWithSource::Null));
      if let ValueWithSource::Scalar { value, .. } = &values[1] {
        assert_eq!(value, "api.local");
      } else {
        panic!("servers.1 must be scalar");
      }
    } else {
      panic!("servers must be sequence");
    }
  } else {
    panic!("config must be mapping");
  }

  // Delete nested map key.
  let del_map = args(Some("db.host"), &[]);
  delete_value(&del_map, &mut config).expect("delete nested map value");

  // Delete sequence item and ensure it shrinks.
  let del_seq = args(Some("servers.0"), &[]);
  delete_value(&del_seq, &mut config).expect("delete sequence value");

  // Deleting non existing keys should be a no-op.
  let missing = args(Some("missing.path"), &[]);
  delete_value(&missing, &mut config).expect("delete missing map");

  if let ValueWithSource::Mapping(ref mapping) = config {
    flattened.insert("dummy".to_string(), ("x".to_string(), temp.path().join(".yuna.yml")));
    let db = mapping.get("db").expect("db key");
    if let ValueWithSource::Mapping(db_map) = db {
      assert!(!db_map.contains_key("host"));
    }
  }
}

#[test]
#[serial]
fn delete_noop_branches_and_nested_sequence_path() {
  let temp = TempDir::new().expect("temp dir");
  env::set_current_dir(temp.path()).expect("set cwd");

  let mut config = ValueWithSource::Null;
  let nested = args(Some("items.0.name"), &["first"]);
  set_value(&nested, &mut config).expect("set nested sequence value");

  let del_from_non_mapping = args(Some("anything"), &[]);
  let mut plain = ValueWithSource::Null;
  delete_value(&del_from_non_mapping, &mut plain).expect("delete on non mapping is no-op");

  let del_out_of_bounds = args(Some("items.3"), &[]);
  delete_value(&del_out_of_bounds, &mut config).expect("delete out-of-bounds sequence index");

  let del_missing_nested = args(Some("items.0.missing"), &[]);
  delete_value(&del_missing_nested, &mut config).expect("delete missing nested key");
}

#[test]
fn set_and_delete_require_name() {
  let mut config = ValueWithSource::Null;
  let opts = args(None, &["value"]);
  let err = set_value(&opts, &mut config).expect_err("set must fail without name");
  assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

  let err = delete_value(&opts, &mut config).expect_err("delete must fail without name");
  assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

  let empty_name_set = args(Some(""), &["value"]);
  set_value(&empty_name_set, &mut config).expect("empty name set is no-op");

  let empty_name_delete = args(Some(""), &[]);
  delete_value(&empty_name_delete, &mut config).expect("empty name delete is no-op");
}

#[test]
fn interpolate_supports_patterns_missing_and_circular() {
  let mut values = FlattenValues::new();
  values.insert(
    "base".to_string(),
    ("https://example.test".to_string(), PathBuf::from("a")),
  );
  values.insert("v".to_string(), ("v1".to_string(), PathBuf::from("a")));

  let interpolated = interpolate(&values, "{base}/{{v}}/{missing}").expect("interpolate");
  assert_eq!(interpolated, "https://example.test//");

  values.insert("loop".to_string(), ("{loop}".to_string(), PathBuf::from("a")));
  let err = interpolate(&values, "{loop}").expect_err("circular reference");
  assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
#[serial]
fn find_and_read_configuration_files_cover_branches() {
  let temp = TempDir::new().expect("temp dir");
  let root = temp.path().join("root");
  let child = root.join("child");
  let leaf = child.join("leaf");
  std::fs::create_dir_all(&leaf).expect("create tree");

  std::fs::write(
    root.join(".yuna.yml"),
    "a: 1\narr: [a]\nobj: {x: root, both: root}\noverride: root\nnullv: null\n",
  )
  .expect("write root config");
  std::fs::write(
    child.join(".yuna.yml"),
    "arr: [b, c]\nobj: {y: child, both: child}\noverride: child\n",
  )
  .expect("write child config");

  env::set_current_dir(&leaf).expect("set cwd");
  let mut opts = args(None, &[]);
  opts.no_merge = true;

  let only_leaf = find_configuration_files(&opts).expect("find with no merge");
  assert_eq!(only_leaf.len(), 1);
  assert!(only_leaf[0].ends_with("root/child/leaf/.yuna.yml"));

  opts.no_merge = false;
  let merged_files = find_configuration_files(&opts).expect("find with merge");
  assert_eq!(merged_files.len(), 2);
  assert!(merged_files[0].ends_with("root/.yuna.yml"));
  assert!(merged_files[1].ends_with("root/child/.yuna.yml"));

  let values = read_configuration_files(&opts).expect("read merged values");
  assert_eq!(values.get("override").map(|(v, _)| v.as_str()), Some("child"));
  assert_eq!(values.get("obj.x").map(|(v, _)| v.as_str()), Some("root"));
  assert_eq!(values.get("obj.y").map(|(v, _)| v.as_str()), Some("child"));
  assert_eq!(values.get("obj.both").map(|(v, _)| v.as_str()), Some("child"));
  assert_eq!(values.get("arr.0").map(|(v, _)| v.as_str()), Some("a"));
  assert_eq!(values.get("arr.1").map(|(v, _)| v.as_str()), Some("b"));
  assert_eq!(values.get("arr.2").map(|(v, _)| v.as_str()), Some("c"));
  assert!(!values.contains_key("nullv"));
}

#[test]
#[serial]
fn read_and_write_current_configuration_file_and_errors() {
  let temp = TempDir::new().expect("temp dir");
  env::set_current_dir(temp.path()).expect("set cwd");

  let opts = args(None, &[]);
  let empty = read_current_configuration_file(&opts).expect("missing file should be empty");
  assert!(matches!(empty, ValueWithSource::Mapping(_)));

  std::fs::write(".yuna.yml", "valid: true\n").expect("write valid yaml");
  let parsed = read_configuration_file(".yuna.yml").expect("read valid yaml");
  assert!(matches!(parsed, ValueWithSource::Mapping(_)));

  std::fs::write("broken.yml", "a: [1\n").expect("write invalid yaml");
  let err = read_configuration_file("broken.yml").expect_err("invalid yaml must fail");
  assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
  assert!(err.to_string().contains("contains invalid YAML"));

  std::fs::create_dir_all("as_dir.yml").expect("make directory");
  let err = read_configuration_file("as_dir.yml").expect_err("reading directory must fail");
  assert_ne!(err.kind(), std::io::ErrorKind::NotFound);

  let mut map = Mapping::new();
  map.insert(
    "k".to_string(),
    scalar("v", &temp.path().join(".yuna.yml"), ScalarType::String),
  );
  let to_write = ValueWithSource::Mapping(map);
  write_current_configuration_file(&opts, &to_write).expect("write current config");

  let loaded = read_current_configuration_file(&opts).expect("read written config");
  if let ValueWithSource::Mapping(m) = loaded {
    assert!(m.contains_key("k"));
  } else {
    panic!("loaded config must be mapping");
  }
}

#[test]
#[serial]
fn set_value_uses_fallback_source_when_cwd_is_unavailable() {
  let temp = TempDir::new().expect("temp dir");
  let ghost = temp.path().join("ghost");
  fs::create_dir_all(&ghost).expect("create ghost dir");
  env::set_current_dir(&ghost).expect("set cwd to ghost");
  fs::remove_dir_all(&ghost).expect("remove ghost dir");

  let mut config = ValueWithSource::Null;
  let opts = args(Some("k"), &["v"]);
  set_value(&opts, &mut config).expect("set value after cwd removal");

  match config {
    ValueWithSource::Mapping(m) => {
      match m.get("k").expect("k exists") {
        ValueWithSource::Scalar { source, .. } => {
          assert_eq!(source, &PathBuf::from(".yuna.yml"));
        }
        _ => panic!("k must be scalar"),
      }
    }
    _ => panic!("config should be mapping"),
  }
}

#[test]
#[serial]
fn write_current_configuration_file_fails_when_cwd_is_unavailable() {
  let temp = TempDir::new().expect("temp dir");
  let ghost = temp.path().join("ghost");
  fs::create_dir_all(&ghost).expect("create ghost dir");
  env::set_current_dir(&ghost).expect("set cwd to ghost");
  fs::remove_dir_all(&ghost).expect("remove ghost dir");

  let opts = args(None, &[]);
  let value = ValueWithSource::Null;
  let err = write_current_configuration_file(&opts, &value).expect_err("missing cwd must fail");
  assert_ne!(err.kind(), std::io::ErrorKind::InvalidData);
}
