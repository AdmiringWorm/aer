// Copyright (c) 2021 Kim J. Nordmo and WormieCorp.
// Licensed under the MIT license. See LICENSE.txt file in the project

pub mod commands;
pub mod logging;

use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::ops::Add;
use std::path::Path;
use std::str::FromStr;

use md5::Md5;
use sha1::Sha1;
use sha2::digest::generic_array::ArrayLength;
use sha2::{Digest, Sha256, Sha512};
use structopt::StructOpt;

#[derive(Debug, PartialEq, StructOpt)]
pub enum ChecksumType {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl FromStr for ChecksumType {
    type Err = &'static str;

    fn from_str(val: &str) -> std::result::Result<Self, <Self as std::str::FromStr>::Err> {
        let val: &str = &val.trim().to_lowercase();

        match val {
            "md5" => Ok(ChecksumType::Md5),
            "sha1" => Ok(ChecksumType::Sha1),
            "sha2" | "sha256" => Ok(ChecksumType::Sha256),
            "sha512" => Ok(ChecksumType::Sha512),
            _ => Err("The value is not a supported checksum type!"),
        }
    }
}

impl Display for ChecksumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            ChecksumType::Md5 => f.write_str("md5"),
            ChecksumType::Sha1 => f.write_str("sha1"),
            ChecksumType::Sha256 => f.write_str("sha256"),
            ChecksumType::Sha512 => f.write_str("sha512"),
        }
    }
}

impl Default for ChecksumType {
    fn default() -> Self {
        Self::Sha256
    }
}

impl ChecksumType {
    pub fn variants() -> &'static [ChecksumType] {
        static VARIANTS: &[ChecksumType] = &[
            ChecksumType::Md5,
            ChecksumType::Sha1,
            ChecksumType::Sha256,
            ChecksumType::Sha512,
        ];

        VARIANTS
    }

    pub fn variants_str() -> &'static [&'static str] {
        static VARIANTS: &[&str] = &["md5", "sha1", "sha256", "sha512"];

        VARIANTS
    }

    pub fn generate(&self, path: &Path) -> Result<String, std::io::Error> {
        generate_checksum(path, self)
    }
}

fn generate_checksum(path: &Path, checksum_type: &ChecksumType) -> Result<String, std::io::Error> {
    match checksum_type {
        ChecksumType::Md5 => generate_checksum_from_hasher(Md5::new(), path),
        ChecksumType::Sha1 => generate_checksum_from_hasher(Sha1::new(), path),
        ChecksumType::Sha256 => generate_checksum_from_hasher(Sha256::new(), path),
        ChecksumType::Sha512 => generate_checksum_from_hasher(Sha512::new(), path),
    }
}

fn generate_checksum_from_hasher<T: Digest + Write>(
    mut hasher: T,
    path: &Path,
) -> Result<String, std::io::Error>
where
    <T as Digest>::OutputSize: Add,
    <<T as Digest>::OutputSize as Add>::Output: ArrayLength<u8>,
{
    let mut f = File::open(path)?;
    std::io::copy(&mut f, &mut hasher)?;
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rstest::rstest;

    use super::*;

    #[test]
    fn default_should_be_sha256() {
        assert_eq!(ChecksumType::default(), ChecksumType::Sha256);
    }

    #[test]
    fn variants_should_return_supported_values() {
        let expected = &[
            ChecksumType::Md5,
            ChecksumType::Sha1,
            ChecksumType::Sha256,
            ChecksumType::Sha512,
        ];

        let actual = ChecksumType::variants();

        assert_eq!(actual, expected);
    }

    #[test]
    fn variants_str_should_return_supported_values_as_a_string() {
        let expected = &["md5", "sha1", "sha256", "sha512"];

        let actual = ChecksumType::variants_str();

        assert_eq!(actual, expected);
    }

    #[rstest(
        test,
        expected,
        case(ChecksumType::Md5, "md5"),
        case(ChecksumType::Sha1, "sha1"),
        case(ChecksumType::Sha256, "sha256"),
        case(ChecksumType::Sha512, "sha512")
    )]
    fn fmt_should_format_checksum_type_in_lowercase(test: ChecksumType, expected: &str) {
        let actual = test.to_string();

        assert_eq!(actual, expected);
    }

    #[rstest(
        test,
        expected,
        case("Md5", ChecksumType::Md5),
        case("sha1", ChecksumType::Sha1),
        case("SHA2", ChecksumType::Sha256),
        case("sha256", ChecksumType::Sha256),
        case("Sha512", ChecksumType::Sha512)
    )]
    fn from_str_should_create_expected_type(test: &str, expected: ChecksumType) {
        let actual = ChecksumType::from_str(test);

        assert_eq!(actual, Ok(expected));
    }

    #[test]
    fn from_str_should_return_error_on_unknown_value() {
        let actual = ChecksumType::from_str("unknown value").unwrap_err();

        assert_eq!(actual, "The value is not a supported checksum type!")
    }

    #[rstest(
        algorithm,
        expected,
        case(ChecksumType::Md5, "ab66430167ceb33784387abe71cf7c7d"),
        case(ChecksumType::Sha1, "86263d6db9edba53dca1cafca3853e2c81983afa"),
        case(ChecksumType::Sha256, "856ee247a62ef795346a4e5f9d1106373a2add6185aa2b2609e6816496c7c839"),
        case(ChecksumType::Sha512, "dfa0d071ed794349d2f67f452a8cb08fcf9f572653cccd193ebd62b5baefd93059d4178615dd7587bd2d6146b9be689418029d28d2d32d7551edc04606a1d204")
    )]
    fn generate_should_generate_correct_checksum(
        algorithm: ChecksumType,
        expected: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from("test-data/checksum-test.bin.txt");

        let actual = algorithm.generate(&path)?;

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn generate_should_return_error_on_non_existing_file() {
        let path = PathBuf::from("non-existing");

        let actual = ChecksumType::default().generate(&path).unwrap_err();

        assert_eq!(actual.kind(), std::io::ErrorKind::NotFound);
    }
}
