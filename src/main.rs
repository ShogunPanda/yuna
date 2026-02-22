use std::borrow::Cow;
use std::io::Error;

use anyhow::{Context, Result};
use clap::Parser;
use comfy_table::{Attribute, Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS};
use yuna::cli::Args;
use yuna::config::{
  delete_value, interpolate, read_configuration_files, read_current_configuration_file, set_value,
  write_current_configuration_file,
};

const TABLE_BORDERS: &str = "││──├─┼┤│    ┬┴┬┴┌┐└┘";

fn show_value(opts: &Args, name: &str) -> Result<(), Error> {
  let values = read_configuration_files(opts)?;

  if let Some(value) = values.get(name) {
    let output = match opts.raw {
      true => Cow::Borrowed(&value.0),
      false => Cow::Owned(interpolate(&values, &value.0)?),
    };
    println!("{}", output);
  }

  Ok(())
}

fn list_values(opts: &Args) -> Result<(), Error> {
  let values = read_configuration_files(opts)?;

  if values.is_empty() {
    println!("No variables found.");
    return Ok(());
  }

  // Build and show the table
  let mut table = Table::new();
  table
    .load_preset(TABLE_BORDERS)
    .apply_modifier(UTF8_ROUND_CORNERS)
    .set_header(vec![
      Cell::new("Name").add_attribute(Attribute::Bold),
      Cell::new("Value").add_attribute(Attribute::Bold),
      Cell::new("Raw").add_attribute(Attribute::Bold),
      Cell::new("Source ").add_attribute(Attribute::Bold),
    ]);

  let cwd_str = opts.cwd.display().to_string();

  for (name, (value, source)) in values.iter() {
    let source = if source.parent() == Some(opts.cwd.as_path()) {
      source
        .file_name()
        .map(|f| f.to_string_lossy())
        .unwrap_or_else(|| source.as_os_str().to_string_lossy())
    } else {
      Cow::Owned(
        source
          .display()
          .to_string()
          .replace(&cwd_str, ".")
          .replace(&opts.home, "~"),
      )
    };

    let interpolated = match opts.raw {
      true => Cow::Borrowed(value),
      false => Cow::Owned(interpolate(&values, value)?),
    };

    table.add_row(vec![
      Cell::new(name).fg(Color::Cyan).add_attribute(Attribute::Bold),
      Cell::new(value).fg(Color::Green).add_attribute(Attribute::Bold),
      Cell::new(interpolated.as_ref()),
      Cell::new(source).fg(Color::Grey),
    ]);
  }

  println!("{}", table);

  Ok(())
}

fn main() -> Result<()> {
  let mut opts = match Args::try_parse() {
    Ok(opts) => opts,
    Err(e) => {
      match e.kind() {
        clap::error::ErrorKind::DisplayVersion => {
          println!("{}", env!("CARGO_PKG_VERSION"));
          return Ok(());
        }
        _ => {
          e.exit();
        }
      }
    }
  };

  opts.cwd = std::env::current_dir()?;
  opts.home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))?; // Linux or Windows

  if opts.delete {
    let mut file = read_current_configuration_file(&opts).context("Cannot read configuration file")?;
    delete_value(&opts, &mut file).context("Cannot delete value")?;
    write_current_configuration_file(&opts, &file).context("Cannot write configuration file")?;
  } else if !opts.value.is_empty() {
    let mut file = read_current_configuration_file(&opts).context("Cannot read configuration file")?;
    set_value(&opts, &mut file).context("Cannot set value")?;
    write_current_configuration_file(&opts, &file).context("Cannot write configuration file")?;
  } else if let Some(ref name) = opts.name {
    show_value(&opts, name).context("Cannot read configuration files")?;
  } else {
    list_values(&opts).context("Cannot read configuration files")?;
  }

  Ok(())
}
