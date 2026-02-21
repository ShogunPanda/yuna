use anyhow::Result;
use regex::Regex;
use serde::de::DeserializeSeed;
use std::{
  collections::HashSet,
  io::{Error, ErrorKind},
  path::{Path, PathBuf},
  sync::LazyLock,
};

use crate::cli::Args;
use crate::serde::{ConfigFile, FlattenValues, Mapping, ScalarType, ValueWithSource};

static INTERPOLATION_PARSER: LazyLock<Regex> =
  LazyLock::new(|| regex::Regex::new(r"\{\{([^\}]+)\}\}|\{([^\}]+)\}").expect("Invalid regex"));

fn merge_values(target: &mut ValueWithSource, source: &ValueWithSource) {
  match (target, source) {
    // Both are mappings, we need to merge them recursively
    (ValueWithSource::Mapping(t), ValueWithSource::Mapping(s)) => {
      for (key, value) in s {
        match t.get_mut(key) {
          Some(existing) => merge_values(existing, value),
          None => {
            t.insert(key.clone(), value.clone());
          }
        }
      }
    }
    // Both are sequences, we concatenate the source to the value
    (ValueWithSource::Sequence(t), ValueWithSource::Sequence(s)) => {
      for item in s {
        t.push(item.clone());
      }
    }
    // Mismatched types or scalars, we override the target with the source
    (t, s) => {
      *t = s.clone();
    }
  }
}

fn flatten_values(values: &mut FlattenValues, mut path: String, current: &ValueWithSource) {
  if path.starts_with('.') {
    path = path[1..].to_string();
  }

  match current {
    ValueWithSource::Mapping(m) => {
      for (key, value) in m {
        flatten_values(values, format!("{}.{}", path, key), value);
      }
    }
    ValueWithSource::Sequence(s) => {
      for (index, item) in s.iter().enumerate() {
        flatten_values(values, format!("{}.{}", path, index), item);
      }
    }
    ValueWithSource::Scalar { value, source, .. } => {
      values.insert(path, (value.clone(), source.clone()));
    }
    _ => (),
  }
}

pub fn set_value(opts: &Args, config: &mut ValueWithSource) -> Result<(), Error> {
  let mut config = config;
  let name = opts
    .name
    .as_ref()
    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Variable name is required for set operation"))?;
  let mut tokens = name.split('.').peekable();
  let new_value = ValueWithSource::Scalar {
    value: opts.value.join(" "),
    source: std::env::current_dir()
      .unwrap_or_else(|_| PathBuf::new())
      .join(&opts.config),
    original_type: ScalarType::String,
  };

  // Until we have tokens
  while let Some(token) = tokens.next() {
    let is_last = tokens.peek().is_none();

    // Numeric, it's a sequence
    if let Ok(index) = token.parse::<usize>() {
      if !config.is_sequence() {
        *config = ValueWithSource::Sequence(vec![]);
      }

      let seq = config.as_sequence_mut().ok_or_else(|| {
        Error::new(
          ErrorKind::InvalidData,
          "Internal error: failed to get sequence after initialization",
        )
      })?;

      // Extend the sequence if necessary
      while seq.len() <= index {
        seq.push(ValueWithSource::Null);
      }

      if is_last {
        seq[index] = new_value;
        return Ok(());
      }

      config = &mut seq[index];
    } else {
      if !config.is_mapping() {
        *config = ValueWithSource::Mapping(Mapping::new());
      }
      let mapping = config.as_mapping_mut().ok_or_else(|| {
        Error::new(
          ErrorKind::InvalidData,
          "Internal error: failed to get mapping after initialization",
        )
      })?;

      let value = mapping
        .entry(token.to_string())
        .or_insert_with(|| ValueWithSource::Mapping(Mapping::new()));

      if is_last {
        *value = new_value;
        return Ok(());
      }

      config = value;
    }
  }
  Ok(())
}

pub fn delete_value(opts: &Args, config: &mut ValueWithSource) -> Result<(), Error> {
  let mut config = config;
  let name = opts.name.as_ref().ok_or_else(|| {
    Error::new(
      ErrorKind::InvalidInput,
      "Variable name is required for delete operation",
    )
  })?;
  let mut tokens = name.split('.').peekable();

  while let Some(token) = tokens.next() {
    let is_last = tokens.peek().is_none();

    // Numeric, it's a sequence
    if let Ok(index) = token.parse::<usize>() {
      let seq = match config.as_sequence_mut() {
        Some(s) if index < s.len() => s,
        _ => return Ok(()), // Key doesn't exist, nothing to delete
      };

      if is_last {
        seq.remove(index);
        return Ok(());
      }

      config = &mut seq[index];
    } else {
      let mapping = match config.as_mapping_mut() {
        Some(m) => m,
        None => return Ok(()), // Key doesn't exist, nothing to delete
      };

      if is_last {
        mapping.remove(token);
        return Ok(());
      }

      match mapping.get_mut(token) {
        Some(value) => config = value,
        None => return Ok(()), // Key doesn't exist, nothing to delete
      }
    }
  }
  Ok(())
}

pub fn interpolate(values: &FlattenValues, value: &str) -> Result<String, Error> {
  let mut value = value.to_string();
  let mut seen = HashSet::new();

  while let Some(captures) = INTERPOLATION_PARSER.captures(&value) {
    let reference = captures
      .get(1)
      .or_else(|| captures.get(2))
      .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid interpolation capture group"))?
      .as_str()
      .to_string();

    if seen.contains(&reference) {
      return Err(Error::new(
        ErrorKind::InvalidData,
        format!("Circular reference detected for key '{}'", reference),
      ));
    }

    let replacement = values.get(&reference).map(|(v, _)| v.as_str()).unwrap_or("");
    value = value.replace(&format!("{{{}}}", reference), replacement);
    seen.insert(reference);
  }

  Ok(value)
}

pub fn find_configuration_files(opts: &Args) -> Result<Vec<PathBuf>, Error> {
  let cwd = std::env::current_dir()?;
  let file = Path::new(&opts.config);

  let mut files = vec![];

  // If no merge, we only care about the current directory
  if opts.no_merge {
    return Ok(vec![cwd.join(file)]);
  }

  let mut current = PathBuf::new();

  // At each level, see if there is a file. We start from the root to the current directory
  for component in cwd.components() {
    current = current.join(component);
    let current_file = current.join(file);

    if current_file.is_file() {
      files.push(current_file);
    }
  }

  Ok(files)
}

pub fn read_configuration_file(config: &str) -> Result<ValueWithSource, Error> {
  let file = Path::new(config);

  match std::fs::read_to_string(file) {
    Ok(c) => {
      let deserializer = serde_yml::Deserializer::from_str(&c);
      let config = ConfigFile {
        source: file.to_path_buf(),
      };

      match config.deserialize(deserializer) {
        Ok(m) => Ok(m),
        Err(e) => Err(Error::new(
          ErrorKind::InvalidData,
          format!("File {} contains invalid YAML: {}", file.display(), e),
        )),
      }
    }
    Err(e) if ErrorKind::NotFound == e.kind() => Ok(ValueWithSource::Mapping(Mapping::new())),
    Err(e) => Err(e),
  }
}

pub fn read_configuration_files(opts: &Args) -> Result<FlattenValues, Error> {
  let files = find_configuration_files(opts)?;
  let mut tree_values = ValueWithSource::Mapping(Mapping::new());

  for file in files {
    let current = read_configuration_file(&file.display().to_string())?;
    merge_values(&mut tree_values, &current);
  }

  let mut values = FlattenValues::new();
  flatten_values(&mut values, "".into(), &tree_values);
  Ok(values)
}

pub fn read_current_configuration_file(opts: &Args) -> Result<ValueWithSource, Error> {
  let cwd = std::env::current_dir()?;
  let path = cwd.join(Path::new(&opts.config));

  let path_str = path.to_str().ok_or_else(|| {
    Error::new(
      ErrorKind::InvalidData,
      format!("Configuration file path contains invalid UTF-8: {:?}", path),
    )
  })?;

  read_configuration_file(path_str)
}

pub fn write_current_configuration_file(opts: &Args, config: &ValueWithSource) -> Result<(), Error> {
  let cwd = std::env::current_dir()?;
  let path = cwd.join(&opts.config);

  let content = serde_yml::to_string(config).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
  std::fs::write(&path, content)
}
