use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::{Attribute, Cell, Color, Table};
use handlebars::Handlebars;
use serde_yaml::{from_str, to_string, Error, Mapping, Value};
use std::fs::{read_to_string, write};
use std::path::{Path, PathBuf};
use std::process::exit;

fn append_mapping<'a>(mapping: &'a mut Mapping, key: &Value) -> &'a mut Mapping {
  mapping.insert(key.clone(), Mapping::new().into());

  mapping.get_mut(key).unwrap().as_mapping_mut().unwrap()
}

fn merge_mappings(destination: &mut Mapping, source: &Mapping) {
  for (key, value) in source {
    // The new value is not a mapping, just ovewrite
    if !value.is_mapping() {
      destination.insert(key.clone(), value.clone());
      continue;
    }

    match destination.get_mut(key) {
      Some(existing) => {
        // There is a value, check if overwrite or merge
        if existing.is_mapping() {
          // If they are both mappings, merge recursively
          merge_mappings(existing.as_mapping_mut().unwrap(), value.as_mapping().unwrap());
        } else {
          // Overwrite
          destination.insert(key.clone(), value.clone());
        }
      }
      _ => {
        // No value, just copy the entire value
        destination.insert(key.clone(), value.clone());
      }
    }
  }
}

fn enumerate_values(values: &mut Vec<(String, String)>, old_path: Vec<String>, current: &Value) {
  match current {
    Value::Mapping(m) => {
      for (key, value) in m {
        // Recurse with the added path
        let mut path = old_path.clone();
        path.push(key.as_str().unwrap().into());
        enumerate_values(values, path, value);
      }
    }
    Value::Sequence(s) => {
      for (key, value) in s.iter().enumerate() {
        // Recurse with the added index as key
        let mut path = old_path.clone();
        path.push(key.to_string());
        enumerate_values(values, path, value);
      }
    }
    // Add the value
    Value::Bool(b) => {
      values.push((old_path.join("."), b.to_string()));
    }
    Value::Number(n) => {
      values.push((old_path.join("."), n.to_string()));
    }
    Value::String(s) => {
      values.push((old_path.join("."), s.into()));
    }
    _ => {
      values.push((old_path.join("."), "null".into()));
    }
  }
}

fn resolve_value(template: &str, values: &Mapping, raw: bool, level: u8) -> Option<String> {
  match Handlebars::new().render_template(template, values) {
    // Template is valid
    Ok(result) => {
      // If there is subtemplate, recurse once
      if result.contains("{{") && !raw && level == 0 {
        resolve_value(&result, values, raw, level + 1)
      // Return the result
      } else if !result.is_empty() {
        Some(result)
      } else {
        None
      }
    }
    Err(e) => {
      if level == 0 {
        eprintln!("Invalid expression: {}", e);
      } else {
        eprintln!("Invalid expression from recursion: {}", e);
      }

      exit(1);
    }
  }
}

fn get_value(values: &Mapping, name: &str, raw: bool) -> String {
  // Always assume it's a handlerbars template
  let mut template = String::from(name);
  template.insert_str(0, "{{{");
  template.push_str("}}}");

  resolve_value(&template, values, raw, 0).unwrap_or_else(String::new)
}

fn list_files(cwd: PathBuf, config: &str) -> Vec<PathBuf> {
  let file = Path::new(config);
  let mut current = PathBuf::new();
  let mut files = vec![];

  // At each level, see if there is a file
  for component in cwd.components() {
    current = current.join(component);
    let current_file = current.join(file);

    if current_file.is_file() {
      files.push(current_file);
    }
  }

  files
}

fn load_file(full_path: PathBuf) -> Mapping {
  // Open the file as YAML
  match read_to_string(&full_path) {
    Ok(c) => {
      let parsed: Result<Mapping, Error> = from_str(&c);

      match parsed {
        Ok(c) => c,
        Err(e) => {
          eprintln!("Cannot parse file {}: {}", full_path.display(), e);
          exit(1);
        }
      }
    }
    Err(e) if std::io::ErrorKind::NotFound == e.kind() => Mapping::new(),
    Err(e) => {
      eprintln!("Cannot read file {}: {}", full_path.display(), e);
      exit(1);
    }
  }
}

fn load_values(config: &str, no_merge: bool) -> Mapping {
  let cwd = match std::env::current_dir() {
    Ok(c) => c,
    Err(e) => {
      eprintln!("Cannot get the current directory: {}", e);
      exit(1);
    }
  };

  let mut values = Mapping::new();

  // Get values from file
  if no_merge {
    let file_values = load_file(cwd.join(config));
    merge_mappings(&mut values, &file_values);
  } else {
    // For each file, load values and merge them
    for file in list_files(cwd, config) {
      let file_values = load_file(file.clone());

      merge_mappings(&mut values, &file_values);
    }
  }

  values
}

fn list_values(config: &str, raw: bool, no_merge: bool) {
  // Get all possible values, in hierarchy, as list
  let values = load_values(config, no_merge);
  let mut all: Vec<(String, String)> = vec![];
  enumerate_values(&mut all, vec![], &values.clone().into());

  // Prepare the table
  let mut table = Table::new();
  table
    .load_preset("││──╞═╪╡│    ┬┴┬┴┌┐└┘")
    .apply_modifier(UTF8_ROUND_CORNERS)
    .set_header(vec![
      Cell::new("Name").add_attribute(Attribute::Bold),
      Cell::new("Current Value").add_attribute(Attribute::Bold),
      Cell::new("Raw Value").add_attribute(Attribute::Bold),
    ]);

  // Create the rows
  for value in all {
    let name = value.0.clone();
    table.add_row(vec![
      Cell::new(value.0).fg(Color::Green).add_attribute(Attribute::Bold),
      Cell::new(if !raw {
        get_value(&values, &name, raw)
      } else {
        "".into()
      })
      .fg(Color::Blue)
      .add_attribute(Attribute::Bold),
      Cell::new(value.1).fg(Color::Yellow),
    ]);
  }

  // Print values
  println!("{}", table);
}

fn read_value(config: &str, name: &str, raw: bool, no_merge: bool) {
  // Load values
  let values = load_values(config, no_merge);

  // Make sure it's a handlebars expressions
  let mut template = String::from(name);

  if !template.contains("{{") {
    template.insert_str(0, "{{{");
    template.push_str("}}}");
  }

  // Resolve the value, then print it
  let resolved = resolve_value(&template, &values, raw, 0);

  if resolved.is_some() {
    println!("{}", resolved.unwrap());
  }
}

fn write_value(config: &str, name: &str, value: &str, delete: bool) {
  // First of all, open the local file, if any
  let cwd = match std::env::current_dir() {
    Ok(c) => c,
    Err(e) => {
      eprintln!("Cannot write {} in the current directory: {}", config, e);
      exit(1);
    }
  };

  // Load current file
  let full_path = cwd.join(config);
  let mut contents = load_file(full_path.clone());
  let mut current_mapping = &mut contents;

  // Split the name and make sure the right structure is present
  let mut tokens = name.split('.').peekable();
  let mut final_key: Value = String::from("").into();

  while let Some(token) = tokens.next() {
    let key = String::from(token).into();

    if tokens.peek().is_none() {
      // We have reached the end, nothing to do here
      final_key = key;
      continue;
    }

    match current_mapping.get(&key) {
      Some(value) => {
        // There is a value, if it was not a mapping overwrite
        current_mapping = if value.is_mapping() {
          current_mapping.get_mut(&key).unwrap().as_mapping_mut().unwrap()
        } else {
          append_mapping(current_mapping, &key)
        };
      }
      _ => {
        // No structure, create
        current_mapping = append_mapping(current_mapping, &key);
      }
    }
  }

  // Write or delete the key
  if delete {
    current_mapping.remove(&final_key);
  } else {
    current_mapping.insert(final_key, value.into());
  }

  // Finally write the file back
  if let Err(e) = write(&full_path, to_string(&contents).unwrap()) {
    eprintln!("Cannot write file {}: {}", full_path.display(), e);
    exit(1);
  }
}

fn main() {
  // Parse the CLI
  let matches = App::new(crate_name!())
    .version(crate_version!())
    .about(crate_description!())
    .args(&[
      Arg::from_usage("-c, --config=<CONFIG> 'The configuration file name'").default_value(".yuna.yml"),
      Arg::from_usage("-d, --delete 'Deletes a variable'"),
      Arg::from_usage("-n, --no-merge 'Do not merge with configuration files in parent folders'"),
      Arg::from_usage("-r, --raw 'Do not perform variables replacement'"),
      Arg::from_usage("[name] 'The variable to manipulate'"),
      Arg::from_usage("[value] 'The value to add/overwrite'").multiple(true),
    ])
    .setting(AppSettings::DontCollapseArgsInUsage)
    .setting(AppSettings::ColorNever)
    .setting(AppSettings::HidePossibleValuesInHelp)
    .setting(AppSettings::TrailingVarArg)
    .setting(AppSettings::UnifiedHelpMessage)
    .get_matches();

  // Resolve options
  let delete = matches.is_present("delete");
  let raw = matches.is_present("raw");
  let no_merge = matches.is_present("no-merge");
  let config = matches.value_of("config").unwrap();
  let name = matches.value_of("name").unwrap_or("");
  let value = matches.values_of("value");

  // Perform the request action
  if let Some(all_values) = value {
    let values = &all_values.collect::<Vec<&str>>().join(" ");
    write_value(config, name, values, delete);
  } else if !name.is_empty() {
    read_value(config, name, raw, no_merge);
  } else {
    list_values(config, raw, no_merge);
  }
}
