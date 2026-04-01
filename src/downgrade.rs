use crate::args::Args;
use crate::config::Config;
use crate::exec;
use crate::util::{input, NumberMenu};

use std::collections::{HashMap, HashSet};
use std::io::{stdin, IsTerminal};

use anyhow::{bail, ensure, Context, Result};
use regex::Regex;
use scraper::{Html, Selector};
use tr::tr;

#[derive(Debug, Clone)]
struct AlaPkg {
    file: String,
    url: String,
    date: Option<String>,
    date_display: Option<String>,
    size: Option<String>,
}

pub async fn run_subcommand<S: AsRef<str>>(config: &mut Config, args: &[S]) -> Result<i32> {
    let mut date = None;
    let mut package = None;
    let mut forwarded = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].as_ref();
        if arg == "--date" {
            i += 1;
            let value = args
                .get(i)
                .map(AsRef::as_ref)
                .context(tr!("--date expects a value"))?;
            date = Some(normalize_ala_date(value)?);
        } else if let Some(value) = arg.strip_prefix("--date=") {
            date = Some(normalize_ala_date(value)?);
        } else if arg.starts_with('-') {
            forwarded.push(arg.to_string());
        } else if package.is_none() {
            package = Some(arg.to_string());
        } else {
            bail!(tr!("downgrade currently supports one package target"));
        }
        i += 1;
    }

    if let Some(pkg) = package {
        let mut init_args = forwarded;
        init_args.push(pkg.clone());
        config.parse_args(init_args)?;
        let pkg = select_package(config, &pkg, date).await?;
        install_from_url(config, &pkg.url)
    } else {
        let date = date.context(tr!("use --date YYYY-MM-DD to downgrade the whole system"))?;
        config.ala_repos = Some(fetch_ala_repos(config, &date).await?);
        let mut cmd = forwarded;
        cmd.push("-S".to_string());
        cmd.push("--repo".to_string());
        cmd.push("--downgrade".to_string());
        cmd.push(date);
        config.parse_args(cmd)?;
        super::handle_cmd(config).await
    }
}

fn install_from_url(config: &mut Config, url: &str) -> Result<i32> {
    config.need_root = true;
    let mut args: Args<&str> = config.pacman_globals();
    args.op("upgrade");
    args.targets.clear();
    args.target(url);
    Ok(exec::pacman(config, &args)?.code())
}

async fn select_package(config: &Config, pkg: &str, date: Option<String>) -> Result<AlaPkg> {
    let mut versions = fetch_pkg_versions(config, pkg).await?;
    ensure!(
        !versions.is_empty(),
        tr!("could not find archived versions for '{}'", pkg)
    );

    versions.sort_by(|a, b| {
        b.date
            .cmp(&a.date)
            .then_with(|| b.file.cmp(&a.file))
            .then_with(|| b.url.cmp(&a.url))
    });

    if let Some(date) = date {
        versions
            .into_iter()
            .find(|v| v.date.as_ref().is_some_and(|d| d <= &date))
            .with_context(|| {
                tr!(
                    "could not find archived versions for '{}' on or before {}",
                    pkg,
                    date
                )
            })
    } else if !stdin().is_terminal() || config.no_confirm {
        Ok(versions.remove(0))
    } else {
        println!("{}", tr!("Choose version for '{}':", pkg));
        let nstyle = config.color.number_menu;
        let vstyle = config.color.sl_version;
        let dstyle = config.color.news_date;
        let sstyle = config.color.stats_value;
        for (i, v) in versions.iter().enumerate() {
            let date = v.date_display.as_deref().unwrap_or("?");
            let ver = pkgver_from_file(pkg, &v.file);
            let size = v.size.as_deref().unwrap_or("?");
            println!(
                "{} {}  {}  {}",
                nstyle.paint(format!("{:>3})", i + 1)),
                vstyle.paint(ver),
                dstyle.paint(date),
                sstyle.paint(size)
            );
        }
        let line = input(config, &tr!("Select one version (eg: 1):"));
        if line.trim().is_empty() {
            return Ok(versions.remove(0));
        }

        let menu = NumberMenu::new(&line);
        let selected = versions
            .iter()
            .enumerate()
            .filter_map(|(i, v)| menu.contains(i + 1, "").then_some(v.clone()))
            .collect::<Vec<_>>();

        ensure!(
            selected.len() == 1,
            tr!("please select exactly one version")
        );
        Ok(selected[0].clone())
    }
}

async fn fetch_pkg_versions(config: &Config, pkg: &str) -> Result<Vec<AlaPkg>> {
    let first = pkg
        .chars()
        .next()
        .context(tr!("package name can not be empty"))?
        .to_ascii_lowercase();
    let index_url = format!("https://archive.archlinux.org/packages/{}/{}/", first, pkg);
    let body = config
        .raur
        .client()
        .get(&index_url)
        .send()
        .await
        .with_context(|| tr!("failed to download {}", index_url))?
        .error_for_status()
        .with_context(|| tr!("failed to download {}", index_url))?
        .text()
        .await?;

    let entry_re = Regex::new(
        r#"href="([^"]+)">[^<]*</a>\s*([0-9]{2}-[A-Za-z]{3}-[0-9]{4})\s+([0-9]{2}:[0-9]{2})\s+(\S+)"#,
    )
    .unwrap();
    let doc = Html::parse_fragment(&body);
    let link_sel = Selector::parse("pre a").unwrap();

    let mut meta = HashMap::new();
    for caps in entry_re.captures_iter(&body) {
        let href = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            let day = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
            let time = caps.get(3).map(|m| m.as_str()).unwrap_or_default();
            let size = caps.get(4).map(|m| m.as_str()).unwrap_or_default();
            if !href.is_empty() {
                let iso = parse_ala_date(day);
                let display = iso
                    .as_ref()
                    .map(|d| d.replace('/', "-"))
                    .zip((!time.is_empty()).then_some(time))
                    .map(|(d, t)| format!("{d} {t}"));
                let size = (!size.is_empty()).then(|| size.to_string());
                meta.insert(href.to_string(), (iso, display, size));
            }
        }

    let mut out = Vec::new();
    for link in doc.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        if !href.starts_with(pkg) || !href.contains(".pkg.tar") || href.ends_with(".sig") {
            continue;
        }
        let (date, date_display, size) = meta
            .get(href)
            .cloned()
            .unwrap_or((None, None, None));
        out.push(AlaPkg {
            file: href.to_string(),
            url: format!("{}{}", index_url, href),
            date,
            date_display,
            size,
        });
    }

    if out.is_empty() {
        bail!(
            tr!(
                "could not find archived versions for '{}' (ALA stores official repo packages only)",
                pkg
            )
        );
    }

    Ok(out)
}

async fn fetch_ala_repos(config: &Config, date: &str) -> Result<HashSet<String>> {
    let url = format!("https://archive.archlinux.org/repos/{}/", date);
    let body = config
        .raur
        .client()
        .get(&url)
        .send()
        .await
        .with_context(|| tr!("failed to download {}", url))?
        .error_for_status()
        .with_context(|| tr!("failed to download {}", url))?
        .text()
        .await?;

    let doc = Html::parse_fragment(&body);
    let link_sel = Selector::parse("pre a").unwrap();
    let mut repos = HashSet::new();

    for link in doc.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        if href == "../" || !href.ends_with('/') {
            continue;
        }
        let name = href.trim_end_matches('/').to_ascii_lowercase();
        if !name.is_empty() {
            repos.insert(name);
        }
    }

    ensure!(
        !repos.is_empty(),
        tr!("could not find repositories in ALA snapshot {}", date)
    );

    Ok(repos)
}

fn pkgver_from_file(pkg: &str, file: &str) -> String {
    let stem = file.split(".pkg.tar").next().unwrap_or(file);
    let no_arch = stem.rsplit_once('-').map(|(v, _)| v).unwrap_or(stem);
    no_arch
        .strip_prefix(&format!("{pkg}-"))
        .unwrap_or(no_arch)
        .to_string()
}

fn parse_ala_date(day: &str) -> Option<String> {
    let mut parts = day.split('-');
    let dd = parts.next()?;
    let mon = parts.next()?;
    let yyyy = parts.next()?;
    if parts.next().is_some() || dd.len() != 2 || yyyy.len() != 4 {
        return None;
    }
    let mm = match mon.to_ascii_lowercase().as_str() {
        "jan" => "01",
        "feb" => "02",
        "mar" => "03",
        "apr" => "04",
        "may" => "05",
        "jun" => "06",
        "jul" => "07",
        "aug" => "08",
        "sep" => "09",
        "oct" => "10",
        "nov" => "11",
        "dec" => "12",
        _ => return None,
    };
    Some(format!("{yyyy}/{mm}/{dd}"))
}

fn normalize_ala_date(s: &str) -> Result<String> {
    const DATE_ERR: &str =
        "invalid ALA date '{}', expected YYYY-MM-DD, YYYY/MM/DD, DD-MM-YYYY or DD/MM/YYYY";
    let sep = if s.contains('-') {
        '-'
    } else if s.contains('/') {
        '/'
    } else {
        bail!(tr!(DATE_ERR, s));
    };

    let mut it = s.split(sep);
    let (y, m, d) = match (it.next(), it.next(), it.next(), it.next()) {
        (Some(y), Some(m), Some(d), None) => (y, m, d),
        _ => {
            bail!(tr!(DATE_ERR, s))
        }
    };

    let (year, month, day) = if y.len() == 4 {
        if !y.chars().all(|c| c.is_ascii_digit())
            || !m.chars().all(|c| c.is_ascii_digit())
            || !d.chars().all(|c| c.is_ascii_digit())
        {
            bail!(tr!(DATE_ERR, s));
        }
        (y.parse::<u32>()?, m.parse::<u32>()?, d.parse::<u32>()?)
    } else if d.len() == 4 {
        if !d.chars().all(|c| c.is_ascii_digit())
            || !m.chars().all(|c| c.is_ascii_digit())
            || !y.chars().all(|c| c.is_ascii_digit())
        {
            bail!(tr!(DATE_ERR, s));
        }
        (d.parse::<u32>()?, m.parse::<u32>()?, y.parse::<u32>()?)
    } else {
        bail!(tr!(DATE_ERR, s));
    };

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        bail!(tr!(DATE_ERR, s));
    }

    Ok(format!("{year:04}/{month:02}/{day:02}"))
}
