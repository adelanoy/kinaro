use anyhow::Result;
use log4rs::Config;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::env;
use std::path::PathBuf;
use log4rs::filter::Response;
use log::Record;

const LOG_FILE: &str = "kinaro.log";

pub fn init(config_dir: &PathBuf) -> Result<()> {
    let file_path = if cfg!(debug_assertions) {
        match env::current_exe() {
            Ok(exe_path) => exe_path.parent().unwrap().to_owned(),
            Err(_) => config_dir.to_owned(),
        }
    } else {
        config_dir.to_owned()
    }
    .join(LOG_FILE);

    let level_filer = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    let console_appender = ConsoleAppender::builder()
        .encoder(Box::new(standard_encoder()))
        .build();
    let file_appender = FileAppender::builder()
        .encoder(Box::new(standard_encoder()))
        .append(false)
        .build(file_path)?;

    let config = Config::builder()
        .appender(Appender::builder().filter(Box::new(KinaroFilter)).build("console", Box::new(console_appender)))
        .appender(Appender::builder().filter(Box::new(KinaroFilter)).build("file", Box::new(file_appender)))
        .build(
            Root::builder()
                .appenders(["console"])
                .build(level_filer),
        )?;

    log4rs::init_config(config)?;
    Ok(())
}

#[derive(Debug)]
struct KinaroFilter;

impl log4rs::filter::Filter for KinaroFilter {
    fn filter(&self, record: &Record) -> Response {
        let target = record.target();
        if target.starts_with("kinaro") || target.starts_with("ki_") {
            Response::Accept
        } else {
            Response::Reject
        }
    }
}

fn standard_encoder() -> PatternEncoder {
    PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} - {f}:{L} - {T} - {h({l})} - {m}{n}")
}
