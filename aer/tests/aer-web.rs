// Copyright (c) 2021 Kim J. Nordmo and WormieCorp.
// Licensed under the MIT license. See LICENSE.txt file in the project

use std::path::PathBuf;
use std::process::Command;

use assert_cmd::prelude::*;
use lazy_static::lazy_static;
use predicates::prelude::*;

#[cfg(feature = "human")]
fn get_readable_bytes<T: Into<f64>>(num: T) -> String {
    human_bytes::human_bytes(num.into())
}

#[cfg(not(feature = "human"))]
fn get_readable_bytes<T: std::fmt::Display>(num: T) -> String {
    format!("{} bytes", num)
}

lazy_static! {
    static ref LOG_DIR: PathBuf = std::env::temp_dir();
}

#[test]
fn should_parse_with_correct_information_command() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aer")?;
    let log_path = LOG_DIR.join("aer-web-tests-parse-correct.log");

    cmd.args(&[
        "web",
        "parse",
        "https://github.com/codecov/codecov-exe/releases",
        "--log",
        log_path.to_str().unwrap(),
    ])
    .env("NO_COLOR", "true");

    cmd.assert().success().stdout(
        predicate::str::contains(
            "https://github.com/codecov/codecov-exe/tree/1.13.0 (type: Unknown, title: 1.13.0, \
             version: None, text: 1.13.0)",
        )
        .and(predicate::str::contains(
            "https://github.com/codecov/codecov-exe/releases/download/1.13.0/codecov-win7-x86.zip \
             (type: Binary, title: None, version: None, text: codecov-win7-x86.zip",
        )),
    );

    Ok(())
}

#[test]
fn should_parse_with_regex_command() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aer")?;
    let log_path = LOG_DIR.join("aer-web-tests-parse-regex.log");

    cmd.args(&[
        "web",
        "parse",
        "https://chocolatey.org",
        "--regex",
        "github",
        "--log",
        log_path.to_str().unwrap(),
    ])
    .env("NO_COLOR", "true");

    cmd.assert().success().stdout(
        predicate::str::contains(
            "https://github.com/chocolatey (type: Unknown, title: None, version: None, text: \
             Chocolatey on GitHub)",
        )
        .and(predicate::str::contains("https://chocolatey.org").count(1)), /* We will just get 1
                                                                            * instance, since it
                                                                            * is the url we
                                                                            * parse */
    );

    Ok(())
}

#[test]
fn should_download_file_and_output_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aer")?;
    let log_path = LOG_DIR.join("aer-web-tests-download-info.log");

    cmd.args(&[
        "web",
        "download",
        "https://github.com/codecov/codecov-exe/releases/download/1.11.0/codecov-linux-x64.zip",
        "--log",
        log_path.to_str().unwrap(),
    ])
    .env("NO_COLOR", "true");

    cmd.assert().success().stdout(
        predicate::str::contains("The web server responded with status: 200 OK!")
            .and(predicate::str::contains("Downloading 'https://github.com/"))
            .and(predicate::str::contains("Successfully downloaded"))
            .and(predicate::str::contains(
                "ETag : a9da76dd5aa96fcee6de685cc1996075",
            ))
            .and(predicate::str::contains(
                "Last Modified : Wed, 10 Jun 2020 06:14:18 GMT",
            ))
            .and(predicate::str::contains(
                "Checksum : bed13834d3203a1511128d19a9595c53364a0ab9f4d7926e6343c41b48b0f6e5",
            ))
            .and(predicate::str::contains("Checksum Type : sha256"))
            .and(predicate::str::contains(format!(
                "The resulting file is {} long!",
                get_readable_bytes(16376743)
            ))),
    );

    Ok(())
}

#[test]
fn should_not_download_up_to_date_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aer")?;
    let log_path = LOG_DIR.join("aer-web-tests-no-download.log");

    cmd.args(&[
        "web",
        "download",
        "https://github.com/chocolatey/ChocolateyGUI/releases/download/0.18.1/ChocolateyGui.Common.0.18.1.nupkg",
        "--etag",
        "f0e303406002b7449f3a92d94761fea6",
        "--last-modified",
        "Mon, 29 Mar 2021 14:28:12 GMT",
        "--log",
        log_path.to_str().unwrap()
    ])
        .env("NO_COLOR", "true");

    cmd.assert().success().stdout(
        predicate::str::contains("The web server responded with status: 304 Not Modified!")
            .and(predicate::str::contains("No download is necessary!")),
    );

    Ok(())
}

#[test]
fn should_keep_downloaded_files() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aer")?;
    let log_path = LOG_DIR.join("aer-web-tests-no-download.log");
    let file_name = "keep-file.exe";
    let work_dir = std::env::temp_dir();
    let full_path = work_dir.join(file_name);
    if full_path.exists() {
        std::fs::remove_file(&full_path)?;
    }

    cmd.args(&[
        "web",
        "download",
        "https://github.com/mwallner/rocolatey/releases/download/v0.5.3/rocolatey-server.exe",
        "--keep-files",
        "--log",
        log_path.to_str().unwrap(),
        "--work-dir",
        work_dir.to_str().unwrap(),
        "--file-name",
        &file_name,
    ])
    .env("NO_COLOR", "true");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "to '{}'",
            full_path.display()
        )));

    assert_eq!(
        true,
        predicate::path::exists()
            .and(predicate::path::is_file())
            .eval(&full_path)
    );

    let _ = std::fs::remove_file(&full_path);

    Ok(())
}

#[test]
fn should_redownload_file_on_checksum_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aer")?;
    let log_path = LOG_DIR.join("aer-web-tests-no-download.log");
    let file_name = "redownload-test.nupkg";
    let work_dir = std::env::temp_dir();
    {
        use std::fs::File;
        use std::io::Write;
        let full_path = work_dir.join(file_name);
        let mut f = File::create(&full_path)?;
        f.write(b"Test File")?;
    }

    cmd.args(&[
        "web",
        "download",
        "https://github.com/cake-contrib/Cake.Recipe/releases/download/2.2.1/Cake.Recipe.2.2.1.nupkg",
        "--log",
        log_path.to_str().unwrap(),
        "--work-dir",
        work_dir.to_str().unwrap(),
        "--file-name",
        &file_name,
        "--checksum",
        "25f3869e37d0b8275adc7f076144705abf30fab676d3d835dbe06cc21a6192e4"
    ])
    .env("NO_COLOR", "true");

    cmd.assert()
        .success()
        .stdout(
            predicate::str::contains("Downloading").and(predicate::str::contains(
                "Original Checksum matches the checksum of the downloaded file!",
            )),
        )
        .stderr(predicate::str::contains(
            "File exists, but do not match the specified checksum.",
        ));

    Ok(())
}

#[test]
fn should_no_download_file_when_checksum_matches() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("aer")?;
    let log_path = LOG_DIR.join("aer-web-tests-no-download.log");
    let file_name = "checksum-match.nupkg";
    let work_dir = std::env::temp_dir();
    let checksum = {
        use std::fs::File;
        use std::io::Write;

        use sha2::{Digest, Sha256};

        let full_path = work_dir.join(file_name);
        {
            let mut f = File::create(&full_path)?;
            f.write(b"Test File")?;
        }
        let mut f = File::open(&full_path)?;

        let mut hasher = Sha256::new();
        std::io::copy(&mut f, &mut hasher)?;
        format!("{:x}", hasher.finalize())
    };

    cmd.args(&[
        "web",
        "download",
        "https://not-really.important",
        "--log",
        log_path.to_str().unwrap(),
        "--work-dir",
        work_dir.to_str().unwrap(),
        "--file-name",
        &file_name,
        "--checksum",
        &checksum,
    ])
    .env("NO_COLOR", "true");

    cmd.assert().success().stdout(predicate::str::contains(
        "File exists, and matches the specified checksum. Nothing to download!",
    ));

    Ok(())
}
