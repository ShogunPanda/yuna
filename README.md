# yuna

[![Version](https://img.shields.io/crates/v/yuna.svg)](https://crates.io/crates/yuna)
[![Dependencies](https://img.shields.io/librariesio/release/cargo/yuna)](https://libraries.io/cargo/yuna)

Naive terminal variables manager.

https://sw.cowtech.it/yuna

## Features

- Store and retrieve variables in YAML format
- Hierarchical configuration with automatic parent directory merging
- Variable interpolation with `{variable}` or `{{variable}}` syntax
- Support for nested objects and arrays
- Clean table output with source tracking
- Simple command-line interface

## Installation

### Prebuilt Binaries

Download one of the prebuilt binaries for your platform, then make it executable:

- [Linux](https://github.com/ShogunPanda/yuna/releases/latest/download/yuna-linux)
- [macOS (Intel)](https://github.com/ShogunPanda/yuna/releases/latest/download/yuna-macos-intel)
- [macOS (Apple Silicon)](https://github.com/ShogunPanda/yuna/releases/latest/download/yuna-macos-arm)
- [Windows](https://github.com/ShogunPanda/yuna/releases/latest/download/yuna.exe)

Example:

```sh
curl -L -o yuna https://github.com/ShogunPanda/yuna/releases/latest/download/yuna-linux
chmod a+x yuna
```

### From Source (Cargo)

If your platform is not available or you prefer to build from source, ensure you have Rust installed, then run:

```sh
cargo install yuna
```

## Usage

### Basic Commands

```sh
# List all variables in the current directory and parent directories
yuna

# Set a variable
yuna my_var "some value"

# Set a variable with multiple words (they will be joined with spaces)
yuna greeting Hello World from Yuna

# Get a variable value
yuna my_var

# Delete a variable
yuna -d my_var
# or
yuna --delete my_var

# Show version
yuna -V
```

### Configuration Files

Yuna stores variables in YAML files (`.yuna.yml` by default). The tool automatically:

- Creates a `.yuna.yml` file in the current directory when you save a variable
- Searches for and merges `.yuna.yml` files from all parent directories up to the root
- Shows which file each variable comes from in the listing

### Nested Variables

You can work with nested structures using dot notation:

```sh
# Set nested object values
yuna database.host localhost
yuna database.port 5432
yuna database.name myapp

# Set array values
yuna servers.0 "server1.example.com"
yuna servers.1 "server2.example.com"

# Read nested values
yuna database.host
```

### Variable Interpolation

Variables can reference other variables using `{var}` or `{{var}}` syntax:

```sh
# Set base variables
yuna base_url "https://api.example.com"
yuna api_version "v1"

# Use interpolation
yuna api_endpoint "{base_url}/{api_version}"

# When reading api_endpoint, it will show: https://api.example.com/v1
yuna api_endpoint
```

### Command-Line Options

```
Options:
  -c, --config <CONFIG>  Configuration file name [default: .yuna.yml]
  -d, --delete           Delete a variable
  -n, --no-merge         Do not merge with configuration files in parent folders
  -r, --raw              Do not perform variable interpolation
  -V, --version          Print version
  -h, --help             Print help
```

### Output Format

When listing variables, Yuna displays a formatted table showing:

- **Name**: Variable name (cyan)
- **Value**: Raw value as stored (green)
- **Raw**: Value with interpolation applied (or raw if -r flag is used)
- **Source**: File where the variable is defined (grey)

## How It Works

### File Hierarchy and Merging

When you run Yuna, it searches for configuration files starting from the root directory down to your current directory, loading each `.yuna.yml` file it finds. Values from files closer to your current directory override those from parent directories. This allows for:

- Global settings at the root level
- Project-specific overrides in project directories
- Local development overrides in subdirectories

Example directory structure:

```
/
├── .yuna.yml                 # Global variables
├── projects/
│   ├── .yuna.yml             # Project-wide variables
│   └── my-app/
│       ├── .yuna.yml         # App-specific variables
│       └── src/
│           └── .yuna.yml     # Source-specific variables
```

### Data Types

Yuna supports all YAML data types:

- **Strings**: Simple text values
- **Numbers**: Integer and floating-point values
- **Booleans**: true/false values
- **Arrays**: Ordered lists accessible via numeric indices
- **Objects**: Nested key-value pairs accessible via dot notation
- **Null**: Empty values

### Variable Priority

When the same variable is defined in multiple configuration files:

1. Files are loaded from root to current directory
2. Later values override earlier ones
3. Nested objects are merged recursively
4. Arrays are concatenated when merging

## Contributing to yuna

- Check out the latest master to make sure the feature hasn't been implemented or the bug hasn't been fixed yet
- Check out the issue tracker to make sure someone already hasn't requested it and/or contributed it
- Fork the project
- Start a feature/bugfix branch
- Commit and push until you are happy with your contribution
- Make sure to add tests for it. This is important so I don't break it in a future version unintentionally.

## Copyright

Copyright (C) 2021 and above Shogun (shogun@cowtech.it).

Licensed under the MIT license, which can be found at https://choosealicense.com/licenses/mit.
