use crate::config::{Config, ConfigEnum};

use std::fs::File;
use std::io::{self, stdout};

use anyhow::{bail, Context, Result};
use tr::tr;

pub fn run_subcommand<S: AsRef<str>>(config: &mut Config, args: &[S]) -> Result<i32> {
    let forwarded = args
        .iter()
        .map(|arg| arg.as_ref().to_string())
        .collect::<Vec<_>>();

    init_history_config(config, &forwarded)?;

    if !config.targets.is_empty() {
        bail!(tr!("history does not take arguments"));
    }

    let log_file = config
        .globals
        .args
        .iter()
        .rev()
        .find(|arg| arg.key == "logfile")
        .and_then(|arg| arg.value.clone())
        .unwrap_or_else(|| config.pacman.log_file.clone());

    let mut file =
        File::open(&log_file).with_context(|| tr!("failed to open pacman log '{}'", log_file))?;
    let mut stdout = stdout().lock();
    io::copy(&mut file, &mut stdout)
        .with_context(|| tr!("failed to read pacman log '{}'", log_file))?;

    Ok(0)
}

fn init_history_config(config: &mut Config, args: &[String]) -> Result<()> {
    let mut iter = args.iter().peekable();
    let mut op_count = 0;
    let mut end_of_ops = false;

    while let Some(arg) = iter.next() {
        let value = iter.peek().map(|arg| arg.as_str());
        if config.parse_arg(arg, value, &mut op_count, &mut end_of_ops)? {
            iter.next();
        }
    }

    config.args.op = config.op.as_str().to_string();
    config.args.targets = config.targets.clone();
    config.args.bin = config.pacman_bin.clone();
    config.globals.op = config.op.as_str().to_string();
    config.globals.bin = config.pacman_bin.clone();
    config.pacman = pacmanconf::Config::with_opts(
        config.pacman_conf_bin.as_deref(),
        config.pacman_conf.as_deref(),
        config.root.as_deref(),
    )?;

    if let Some(ref dbpath) = config.db_path {
        config.pacman.db_path = dbpath.clone();
    }

    Ok(())
}
