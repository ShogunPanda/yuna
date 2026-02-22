use std::fs;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn version_and_invalid_args_paths() {
  Command::cargo_bin("yuna")
    .expect("binary")
    .arg("-V")
    .assert()
    .success()
    .stdout(format!("{}\n", env!("CARGO_PKG_VERSION")));

  Command::cargo_bin("yuna")
    .expect("binary")
    .arg("--definitely-invalid")
    .assert()
    .failure();
}

#[test]
fn list_empty_and_set_get_raw_delete_flow() {
  let temp = TempDir::new().expect("temp dir");

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(temp.path())
    .env("HOME", temp.path())
    .assert()
    .success()
    .stdout("No variables found.\n");

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(temp.path())
    .env("HOME", temp.path())
    .args(["base", "https://example.test"])
    .assert()
    .success();

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(temp.path())
    .env("HOME", temp.path())
    .args(["endpoint", "{base}", "v1"])
    .assert()
    .success();

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(temp.path())
    .env("HOME", temp.path())
    .arg("endpoint")
    .assert()
    .success()
    .stdout("https://example.test v1\n");

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(temp.path())
    .env("HOME", temp.path())
    .args(["-r", "endpoint"])
    .assert()
    .success()
    .stdout("{base} v1\n");

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(temp.path())
    .env("HOME", temp.path())
    .assert()
    .success()
    .stdout(contains("Name").and(contains("endpoint")).and(contains(".yuna.yml")));

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(temp.path())
    .env("HOME", temp.path())
    .args(["-d", "endpoint"])
    .assert()
    .success();

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(temp.path())
    .env("HOME", temp.path())
    .arg("endpoint")
    .assert()
    .success()
    .stdout("");
}

#[test]
fn list_parent_source_and_userprofile_fallback() {
  let temp = TempDir::new().expect("temp dir");
  let child = temp.path().join("child");
  fs::create_dir_all(&child).expect("create child");
  fs::write(temp.path().join(".yuna.yml"), "from_parent: hello\n").expect("write parent file");

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(&child)
    .env_remove("HOME")
    .env("USERPROFILE", temp.path())
    .assert()
    .success()
    .stdout(contains("from_parent").and(contains("~")));

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(&child)
    .env("HOME", temp.path())
    .arg("-r")
    .assert()
    .success()
    .stdout(contains("from_parent"));

  Command::cargo_bin("yuna")
    .expect("binary")
    .current_dir(&child)
    .env("HOME", temp.path())
    .arg("-d")
    .assert()
    .failure();
}
