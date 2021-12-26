# yuna

[![Version](https://img.shields.io/crates/v/yuna.svg)](https://crates.io/crates/yuna)
[![Dependencies](https://img.shields.io/librariesio/release/cargo/yuna)](https://libraries.io/cargo/yuna)

Naive terminal variables manager.

https://sw.cowtech.it/yuna

## Install

Download one of the prebuilt binaries for your platform, then make it executable:

- [Linux](https://github.com/ShogunPanda/yuna/releases/latest/download/yuna-linux)
- [macOS (Intel)](https://github.com/ShogunPanda/yuna/releases/latest/download/yuna-macos-intel)
- [macOS (Apple M1)](https://github.com/ShogunPanda/yuna/releases/latest/download/yuna-macos-arm)
- [Windows](https://github.com/ShogunPanda/yuna/releases/latest/download/yuna.exe)

Example:

```sh
curl -L -o yuna https://github.com/ShogunPanda/yuna/releases/latest/download/yuna-linux
chmod a+x yuna
```

## Install (other platforms)

If your platform is not available or you don't want to use the binaries above, make sure you have a Rust installation locally, then run:

```sh
cargo install yuna
```

## Usage

```sh
# If run without arguments, it will list all available variables in that folder.
yuna

# If run with more than an argument, it will write a variable,
yuna foo bar baz

# If run with a argument, it will read a variable.
yuna foo
```

Every time a variable is saved, a `.yuna.yml` is created in the current directory. When searching all available variables, yuna will search and load all `.yuna.yml` files in all ancestors folders, up to `/`.

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
