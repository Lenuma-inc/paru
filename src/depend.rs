use crate::config::Config;

use alpm::Package;
use alpm_utils::{DbListExt, Targ};
use anyhow::{bail, Context, Result};
use tr::tr;

#[derive(Debug, Clone, Copy)]
pub enum Subcommand {
    Depend,
    Provide,
}

pub async fn run_subcommand<S: AsRef<str>>(
    config: &mut Config,
    subcommand: Subcommand,
    args: &[S],
) -> Result<i32> {
    let mut forwarded = Vec::new();
    let mut targets = Vec::new();

    for arg in args {
        let arg = arg.as_ref();
        if arg.starts_with('-') {
            forwarded.push(arg.to_string());
        } else {
            targets.push(arg.to_string());
        }
    }

    if targets.is_empty() {
        bail!(tr!("no targets specified (use -h for help)"));
    }

    let mut init_args = vec!["-Q".to_string()];
    init_args.extend(forwarded);
    config.parse_args(init_args)?;

    let mut ret = 0;
    for target in targets {
        match find_pkg(config, &target) {
            Ok(pkg) => print_pkg(subcommand, pkg),
            Err(err) => {
                eprintln!("{} {}", config.color.error.paint("error:"), err);
                ret = 1;
            }
        }
    }

    Ok(ret)
}

fn find_pkg<'a>(config: &'a Config, target: &str) -> Result<&'a Package> {
    let targ = Targ::from(target);
    if let Some(repo) = targ.repo {
        if repo == "local" {
            return config
                .alpm
                .localdb()
                .pkg(targ.pkg)
                .with_context(|| tr!("package '{}' was not found", target));
        }

        let db = config
            .alpm
            .syncdbs()
            .iter()
            .find(|db| db.name() == repo)
            .with_context(|| tr!("repository '{}' was not found", repo))?;
        return db
            .pkg(targ.pkg)
            .with_context(|| tr!("package '{}' was not found", target));
    }

    config
        .alpm
        .syncdbs()
        .pkg(targ.pkg)
        .or_else(|_| config.alpm.localdb().pkg(targ.pkg))
        .with_context(|| tr!("package '{}' was not found", target))
}

fn print_pkg(subcommand: Subcommand, pkg: &Package) {
    match subcommand {
        Subcommand::Depend => {
            println!("{}", tr!("Package depends on"));
            for dep in pkg.depends() {
                println!("  - {}", dep);
            }
        }
        Subcommand::Provide => {
            println!("{}", tr!("Package provides"));
            for provide in pkg.provides() {
                println!("  - {}", provide);
            }
        }
    }
}
