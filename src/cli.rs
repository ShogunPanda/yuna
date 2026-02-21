use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
  #[arg(skip)]
  pub cwd: std::path::PathBuf,

  #[arg(skip)]
  pub home: String,

  #[arg(short, long, default_value = ".yuna.yml", help = "The configuration file name")]
  pub config: String,

  #[arg(short, long, help = "Deletes a variable")]
  pub delete: bool,

  #[arg(short = 'n', long, help = "Do not merge with configuration files in parent folders")]
  pub no_merge: bool,

  #[arg(short, long, help = "Do not perform variables replacement")]
  pub raw: bool,

  #[arg(help = "The variable to manipulate")]
  pub name: Option<String>,

  #[arg(help = "The value to add/overwrite", num_args = 0..)]
  pub value: Vec<String>,
}
