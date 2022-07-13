use log4rs::append::{console::ConsoleAppender, file::FileAppender};
use log4rs::filter::threshold::ThresholdFilter;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Root};
use log4rs::Config;
use log::LevelFilter;

pub fn initialize() {
    let _ = std::fs::remove_file("output.log");

    //let log_format = "{d(%Y-%m-%d %H:%M:%S)} [{l}] in {f}:{L} {{{t}}}\n{m}\n";
    //let log_format = "[{l}] in {f}:{L} {{{t}}}\n{m}\n";

    let log_format = "{m}\n";

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(log_format)))
        .build();

    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(log_format)))
        .build("output.log")
        .unwrap();

    let config = Config::builder()
        // console appender
        .appender(Appender::builder()
            .filter(Box::new(ThresholdFilter::new(log::LevelFilter::Trace)))
            .build("stdout", Box::new(stdout))
        )
        // file appender
        .appender(Appender::builder()
            .build("file", Box::new(log_file))
        )
        // build
        .build(Root::builder()
                .appenders(["stdout", "file"])
                .build(LevelFilter::Trace))
                .unwrap();

    log4rs::init_config(config).unwrap();
}
