use std::{
  error::Error as StdError,
  fs::{self, File},
  path::PathBuf,
  result::Result as StdResult,
};

use chrono::Local;

use fern::{
  colors::{Color, ColoredLevelConfig},
  Dispatch,
};

pub use log::Level as LogLevel;
use log::LevelFilter;

pub type Result<T> = StdResult<T, Box<dyn StdError>>;

const DEFAULT_MAX_FILE_SIZE: u128 = 1024 * 1024;

/// Targets of the logs.
pub enum LogTarget {
  /// Log to console.
  Console,
  /// Log to the specified dir.
  Dir(PathBuf),
}

pub struct Logger {
  level: LevelFilter,
  max_file_size: u128,
  targets: Vec<LogTarget>,
}

pub struct LoggerBuilder {
  level: LevelFilter,
  max_file_size: u128,
  targets: Vec<LogTarget>,
}

impl LoggerBuilder {
  pub fn new() -> Self {
    Self { level: LevelFilter::Trace, max_file_size: DEFAULT_MAX_FILE_SIZE, targets: Vec::new() }
  }

  pub fn level(mut self, level: LogLevel) -> Self {
    self.level = level.to_level_filter();
    self
  }

  pub fn max_file_size(mut self, max_file_size: u128) -> Self {
    self.max_file_size = max_file_size;
    self
  }

  pub fn targets<T: IntoIterator<Item = LogTarget>>(mut self, targets: T) -> Self {
    for target in targets {
      self.targets.push(target);
    }
    self
  }

  pub fn build(self) -> Result<Logger> {
    let logger =
      Logger { level: self.level, max_file_size: self.max_file_size, targets: self.targets };
    Self::apply(&logger)?;
    Ok(logger)
  }

  fn apply(logger: &Logger) -> Result<()> {
    let colors = ColoredLevelConfig::new().info(Color::Blue).warn(Color::Yellow).error(Color::Red);
    let mut dispatch = Dispatch::new()
      .format(move |out, message, record| {
        let line = match record.line() {
          Some(line) => line,
          None => 0,
        };
        out.finish(format_args!(
          "[{}]<{}>[{}:{}] {}",
          Local::now().format("%Y-%m-%d %H:%M:%S"),
          colors.color(record.level()),
          record.target(),
          line,
          message
        ))
      })
      .level(logger.level);

    for target in &logger.targets {
      dispatch = match target {
        LogTarget::Console => dispatch.chain(std::io::stdout()),
        LogTarget::Dir(dir) => {
          if !dir.exists() {
            fs::create_dir_all(&dir).unwrap();
          }
          let path = Self::get_log_path(dir);
          Self::rotate_file(dir, logger.max_file_size)?;
          dispatch.chain(fern::log_file(path)?)
        }
      };
    }

    dispatch.apply()?;

    Ok(())
  }

  fn rotate_file(dir: &PathBuf, max_file_size: u128) -> Result<()> {
    let path = Self::get_log_path(dir);
    if path.exists() {
      let log_size = File::open(&path)?.metadata()?.len() as u128;
      if log_size > max_file_size {
        let old_path = Self::get_old_log_path(dir);
        if old_path.exists() {
          fs::remove_file(&old_path)?;
        }
        fs::rename(&path, &old_path)?;
      }
    }
    Ok(())
  }

  fn get_log_path(dir: &PathBuf) -> PathBuf {
    dir.join("app.log")
  }

  fn get_old_log_path(dir: &PathBuf) -> PathBuf {
    dir.join("app.log.old")
  }
}
