// Copyright (c) 2021 Kim J. Nordmo and WormieCorp.
// Licensed under the MIT license. See LICENSE.txt file in the project

use std::path::Path;

use log::{debug, Level, LevelFilter};
use yansi::{Color, Paint, Style};

#[macro_export]
macro_rules! log_data {
    () => {
        log_data!{env!("CARGO_PKG_NAME")}
    };
    ($app_name:expr) => {
        #[derive(::structopt::StructOpt)]
        pub struct LogData {
            /// The path to where logs should be written.
            #[structopt(long = "log", alias = "log-file", env = "AER_LOG_PATH", global = true, parse(from_os_str), default_value = concat!("./", $app_name, ".log"))]
            pub path: ::std::path::PathBuf,
            /// The log level to use when outputting to the console.
            #[structopt(short = "-L", long = "log-level", env = "AER_LOG_LEVEL", global = true, default_value = "info", possible_values = &["trace", "debug", "info", "error" ])]
            pub level: ::log::LevelFilter,
        }

        impl Default for LogData {
            fn default() -> Self {
                Self {
                    path: ::std::path::PathBuf::from(concat!("./", $app_name, ".log")),
                    level: ::log::LevelFilter::Info
                }
             }
        }

        impl crate::logging::LogDataTrait for LogData {
            fn path(&self) -> &::std::path::Path { &self.path }
            fn level(&self) -> &::log::LevelFilter { &self.level }
        }
    };
}

pub trait LogDataTrait {
    fn path(&self) -> &Path;
    fn level(&self) -> &LevelFilter;
}

#[derive(Copy, Clone)]
struct Colors {
    trace: Style,
    debug: Style,
    info: Style,
    warn: Style,
    error: Style,
}

impl Colors {
    fn from_level(&self, level: &Level) -> &Style {
        match level {
            Level::Trace => &self.trace,
            Level::Debug => &self.debug,
            Level::Warn => &self.warn,
            Level::Error => &self.error,
            _ => &self.info,
        }
    }

    fn paint<T>(&self, level: &Level, value: T) -> Paint<T> {
        let style = self.from_level(level);

        style.paint(value)
    }

    fn paint_level(&self, level: Level) -> Paint<Level> {
        self.paint(&level, level)
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            trace: Style::new(Color::Black),
            debug: Style::new(Color::Fixed(7)),
            info: Style::new(Color::Unset),
            warn: Style::new(Color::Fixed(208)).bold(),
            error: Style::new(Color::Red).bold(),
        }
    }
}

pub fn setup_logging<T: LogDataTrait>(log: &T) -> Result<(), Box<dyn std::error::Error>> {
    let colors = Colors::default();

    let cli_dispatch = configure_cli_dispatch(colors, log);

    if log.path().exists() {
        let _ = std::fs::remove_file(log.path());
    }

    let mut file_log = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] {} T[{:?}] [{}] {}:{}: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
                record.level(),
                std::thread::current().name().unwrap_or("<unnamed>"),
                record.module_path().unwrap_or("<unnamed>"),
                record.file().unwrap_or("<unnamed>"),
                record.line().unwrap_or(0),
                Paint::wrapping(message).wrap()
            ));
        })
        .level(LevelFilter::Trace);

    for level in get_levels() {
        file_log = file_log.level_for(level.0, level.1);
    }
    file_log = file_log.chain(fern::log_file(log.path())?);

    fern::Dispatch::new()
        .chain(cli_dispatch)
        .chain(file_log)
        .apply()?;

    debug!("Finished configuring logging");

    Ok(())
}

fn configure_cli_dispatch<T: LogDataTrait>(colors: Colors, log: &T) -> fern::Dispatch {
    let mut cli_info = if log.level() > &LevelFilter::Info {
        fern::Dispatch::new().format(move |out, message, record| {
            let level = record.level();
            out.finish(format_args!(
                "[{}]: {}",
                colors.paint_level(level),
                colors.paint(&level, message)
            ));
        })
    } else {
        fern::Dispatch::new().format(move |out, message, record| {
            out.finish(format_args!("{}", colors.paint(&record.level(), message)))
        })
    }
    .filter(move |metadata| metadata.level() >= Level::Info)
    .level(*log.level());

    if log.level() > &LevelFilter::Info {
        for level in get_levels() {
            cli_info = cli_info.level_for(level.0, level.1);
        }
    }

    cli_info = cli_info.chain(std::io::stdout());

    fern::Dispatch::new().chain(cli_info).chain(
        fern::Dispatch::new()
            .format(move |out, message, record| {
                let level = record.level();
                out.finish(format_args!(
                    "[{}]: {}",
                    colors.paint_level(level),
                    colors.paint(&level, message)
                ));
            })
            .filter(move |metadata| metadata.level() <= Level::Warn)
            .level(*log.level())
            .chain(std::io::stderr()),
    )
}

fn get_levels() -> &'static [(&'static str, LevelFilter)] {
    &[
        ("html5ever", LevelFilter::Info),
        ("rustls::client::hs", LevelFilter::Debug),
        ("rustls::client::tls13", LevelFilter::Debug),
        ("tokio_util::codec::framed_impl", LevelFilter::Debug),
        ("reqwest::blocking::wait", LevelFilter::Debug),
    ]
}
