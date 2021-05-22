// Copyright (c) 2021 Kim J. Nordmo and WormieCorp.
// Licensed under the MIT license. See LICENSE.txt file in the project

use std::fmt::Display;
use std::path::PathBuf;

use aer_upd::data::Url;
use aer_upd::web::errors::WebError;
use aer_upd::web::{BinaryResponse, LinkElement, LinkType, ResponseType, WebRequest, WebResponse};
use lazy_static::lazy_static;
use log::{error, info, warn};
use structopt::StructOpt;
use yansi::{Color, Style};

use crate::ChecksumType;

#[derive(StructOpt)]
#[structopt(after_help = "EXAMPLES:
    Parsing all urls
      `parse https://github.com/codecove/codecov-exe/releases/latest`Parsing \
                          on matching urls
      `parse https://github.com/codecove/codecov-exe/releases/latest \
                          --regex '.*\\.zip$'`
    Parsing while extracting version
      `parse https://github.com/codecov/codecov-exe/releases/latest \
                          --regex '/(?P<version>[\\d\\.]+)/.*\\.zip$'`")]
pub struct ParseArguments {
    /// The url to use to test parsing a single web page.
    url: Url,

    /// The regular expression to use when parsing the specified `url`.
    #[structopt(long, short)]
    regex: Option<String>,
}

#[derive(StructOpt)]
pub struct DownloadArguments {
    /// The url of the binary file to download.
    url: Url,

    /// Keep any downloaded file instead of the normal procedure of deleting
    /// them at the end validation.
    #[structopt(long)]
    keep_files: bool,

    /// The etag that will be matched against the download location. If matched
    /// and the server returns a Not Modified response, then no file will be
    /// downloaded.
    #[structopt(long, short)]
    etag: Option<String>,

    /// The last modified date as a string, this is usually the date that has
    /// been previously returned by a server. If this date matches and the
    /// server responds with a Not Modified response, then no file will be
    /// downloaded.
    #[structopt(long, short)]
    last_modified: Option<String>,

    /// The file name to use when downloading a file. This can be used to
    /// override the default name, or if a name can not be detected.
    #[structopt(long)]
    file_name: Option<String>,

    /// The checksum to compare the downladed file with. If an existing file
    /// with the a matching name and it matches the checksum, then a download
    /// will not occurr (*NOT IMPLEMENTED*).
    #[structopt(long, short)]
    checksum: Option<String>,

    /// The type of the checksum to use when comparing and/or outputting to the
    /// console.
    #[structopt(long, default_value, possible_values = ChecksumType::variants_str(), env = "AER_CHECKSUM_TYPE")]
    checksum_type: ChecksumType,

    /// The directory to use when downloading the files. NOTE: This directory
    /// must exist. [default: %TEMP%]
    #[structopt(long, parse(from_os_str))]
    work_dir: Option<PathBuf>,
}

#[derive(StructOpt)]
pub enum WebCommands {
    /// Allows testing a single parse command using the specified url, and
    /// optionally an regex. This will output any links found on the website.
    Parse(ParseArguments),
    /// Allows downloading a single binary file, by defailt this command will
    /// use `%TEMP%` as the work directory and will remove the downladed file
    /// afterwards.
    Download(DownloadArguments),
}

/// Allows testing different web related tasks. The currently supported tasks
/// included the ability to parse HTML websites, and downloading binary files.
#[derive(StructOpt)]
#[structopt(author = env!("CARGO_PKG_AUTHORS"), name = "aer-web")]
pub struct WebArguments {
    #[structopt(subcommand)]
    cmd: WebCommands,
}

pub fn run_web(args: WebArguments) -> Result<(), Box<dyn std::error::Error>> {
    let request = WebRequest::create();
    match args.cmd {
        WebCommands::Parse(args) => parse_cmd(request, args),
        WebCommands::Download(args) => Ok(download_cmd(request, args)?),
    }
}

fn parse_cmd(request: WebRequest, args: ParseArguments) -> Result<(), Box<dyn std::error::Error>> {
    let (parent, links) = parse_website(request, args.url, args.regex)?;
    info!(
        "Successfully parsed '{}'",
        Color::Magenta.paint(parent.link)
    );

    for link in &links {
        info!(
            "{} (type: {}, title: {}, version: {}, text: {})",
            Color::Magenta.paint(&link.link),
            Color::Cyan.paint(link.link_type),
            Color::Cyan.paint(if link.title.is_empty() {
                "None"
            } else {
                &link.title
            }),
            Color::Cyan.paint(if let Some(ref version) = link.version {
                version.to_string()
            } else {
                "None".into()
            }),
            Color::Cyan.paint(&link.text)
        );
    }

    info!(
        "Found {} links on the webpage!",
        Color::Cyan.paint(links.len())
    );

    info!("The following link types was found!");
    for link_type in LinkType::variants() {
        let count = links.iter().filter(|l| l.link_type == *link_type).count();

        info!("Found {:2} {} types!", Color::Cyan.paint(count), link_type);
    }

    Ok(())
}

fn download_cmd(
    request: WebRequest,
    mut args: DownloadArguments,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = if let Some(work_dir) = args.work_dir {
        work_dir
    } else {
        std::env::temp_dir()
    };
    args.work_dir = Some(temp_dir);
    download_file(request, args)
}

fn parse_website(
    request: WebRequest,
    url: Url,
    regex: Option<String>,
) -> Result<(LinkElement, Vec<LinkElement>), WebError> {
    let response = request.get_html_response(url.as_str())?;

    if let Some(ref regex) = regex {
        response.read(Some(regex))
    } else {
        response.read(None)
    }
}

fn download_file(
    request: WebRequest,
    args: DownloadArguments,
) -> Result<(), Box<dyn std::error::Error>> {
    let etag = if let Some(ref etag) = args.etag {
        Some(etag.as_str())
    } else {
        None
    };
    let last_modified = if let Some(ref last_modified) = args.last_modified {
        Some(last_modified.as_str())
    } else {
        None
    };

    if let Some(ref file_name) = args.file_name {
        if validate_local_file(&args, file_name)? {
            return Ok(());
        }
    }

    let response = request.get_binary_response(args.url.as_str(), etag, last_modified)?;

    match response {
        ResponseType::Updated(_) => {
            info!("No download is necessary!");
        }
        ResponseType::New(mut response, _) => {
            handle_new_response(response, args)?;
        }
    }

    Ok(())
}

fn validate_local_file(args: &DownloadArguments, file_name: &str) -> Result<bool, WebError> {
    let checksum = if let Some(ref checksum) = args.checksum {
        checksum
    } else {
        return Ok(false);
    };

    let work_dir = args.work_dir.as_ref().unwrap();
    let possible_path = work_dir.join(file_name);

    if possible_path.exists() {
        let file_checksum = args.checksum_type.generate(&possible_path)?;
        if checksum.to_lowercase() == file_checksum {
            info!("File exists, and matches the specified checksum. Nothing to download!");
            return Ok(true);
        } else {
            warn!("File exists, but do not match the specified checksum. Re-downloading file!");
        }
    } else {
        info!("File exists, but no checksum available. Continuing download!");
    }

    Ok(false)
}

fn handle_new_response(
    mut response: BinaryResponse,
    args: DownloadArguments,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.file_name.is_none() {
        let file_name = response.file_name().unwrap();
        if validate_local_file(&args, &file_name)? {
            info!("Local file already matches the file requested upstream!");
            return Ok(());
        }
    }

    response.set_work_dir(&args.work_dir.unwrap());

    let (etag, last_modified) = get_response_info(&response);
    let result = if let Some(file_name) = args.file_name {
        let file_name_str = Some(file_name.as_str());
        response.read(file_name_str)
    } else {
        response.read(None)
    }?;

    info!("The following information was given by the server:");
    print_string("ETag", etag.trim_matches('"'));
    print_string("Last Modified", &last_modified);

    match args.checksum_type.generate(&result) {
        Ok(checksum) => {
            print_line("Checksum", &checksum);
            print_line("Checksum Type", args.checksum_type);

            if let Some(original_checksum) = args.checksum {
                if original_checksum.to_lowercase() == checksum {
                    info!(
                        "{}",
                        Color::Green.paint(
                            "Original Checksum matches the checksum of the downloaded file!"
                        )
                    );
                } else {
                    warn!("Original Checksum did not match the checksum of the downloaded file!");
                }
            }
        }
        Err(err) => {
            error!("Unable to generate checksum");
            return Err(Box::new(err));
        }
    }

    let len = {
        #[cfg(feature = "human")]
        {
            human_bytes::human_bytes(result.metadata()?.len() as f64)
        }
        #[cfg(not(feature = "human"))]
        format!("{} bytes", result.metadata()?.len())
    };

    info!("The resulting file is {} long!", Color::Cyan.paint(len));

    if !args.keep_files {
        let _ = std::fs::remove_file(result);
    }

    Ok(())
}

fn get_response_info<T: WebResponse>(response: &T) -> (String, String) {
    let headers = response.get_headers();
    let mut etag = String::new();
    let mut last_modified = String::new();

    if let Some(etag_val) = headers.get("etag") {
        etag = etag_val.to_string();
    }
    if let Some(modified_val) = headers.get("last-modified") {
        last_modified = modified_val.to_string();
    }

    (etag, last_modified)
}

fn print_line<T: Display, V: Display>(name: T, value: V) {
    lazy_static! {
        static ref NAME_STYLE: Style = Color::Magenta.style();
        static ref VALUE_STYLE: Style = Color::Cyan.style();
    };

    info!(
        "{:>18} : {}",
        NAME_STYLE.paint(name),
        VALUE_STYLE.paint(value)
    );
}

fn print_string<T: Display>(name: T, value: &str) {
    if value.is_empty() {
        print_line(name, "None");
    } else {
        print_line(name, value);
    }
}
