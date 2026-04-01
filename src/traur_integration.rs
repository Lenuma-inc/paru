//! Integration with traur for automatic PKGBUILD security scanning.

use std::fs;
use std::path::Path;

use ansiterm::Style;
use anyhow::{Context, Result};
use tr::tr;

use crate::config::Config;
use crate::security_scan::{scan_pkgbuild, Tier};

/// Result of a traur security scan for a single package.
#[derive(Debug, Clone)]
pub struct SecurityScanResult {
    pub package_name: String,
    pub is_safe: bool,
    pub tier: String,
    pub score: f64,
    pub signals: Vec<SecuritySignal>,
}

/// Individual security signal detected by traur.
#[derive(Debug, Clone)]
pub struct SecuritySignal {
    pub id: String,
    pub description: String,
    pub points: f64,
    pub category: String,
    pub override_gate: bool,
}

/// Scan a PKGBUILD directory using traur library.
pub fn scan_pkgbuild_dir(_config: &Config, pkg_name: &str, pkg_dir: &Path) -> Result<SecurityScanResult> {
    let pkgbuild_path = pkg_dir.join("PKGBUILD");

    if !pkgbuild_path.exists() {
        return Ok(SecurityScanResult {
            package_name: pkg_name.to_string(),
            is_safe: false,
            tier: "UNKNOWN".to_string(),
            score: 100.0,
            signals: vec![SecuritySignal {
                id: "P-NO-PKGBUILD".to_string(),
                description: tr!("PKGBUILD file not found").to_string(),
                points: 100.0,
                category: "Pkgbuild".to_string(),
                override_gate: false,
            }],
        });
    }

    // Read PKGBUILD content
    let pkgbuild_content = fs::read_to_string(&pkgbuild_path)
        .with_context(|| tr!("failed to read PKGBUILD for '{}'", pkg_name))?;

    let scan_result = scan_pkgbuild(pkg_name, &pkgbuild_content);

    let signals = scan_result
        .signals
        .iter()
        .map(|sig| SecuritySignal {
            id: sig.id.clone(),
            description: sig.description.clone(),
            points: sig.points,
            category: sig.category.clone(),
            override_gate: sig.override_gate,
        })
        .collect();

    // Package is safe if tier is Trusted/Ok AND no high-severity signals
    let has_high_severity = scan_result
        .signals
        .iter()
        .any(|s| s.points >= 50.0 || s.override_gate);
    let is_safe = scan_result.tier >= Tier::Trusted
        && scan_result.tier <= Tier::Ok
        && !has_high_severity;

    Ok(SecurityScanResult {
        package_name: pkg_name.to_string(),
        is_safe,
        tier: format!("{:?}", scan_result.tier),
        score: scan_result.score as f64,
        signals,
    })
}

/// Scan multiple PKGBUILDs and print results.
pub fn scan_and_print(
    config: &Config,
    packages: &[(&str, &Path)],
) -> Result<bool> {
    let c = config.color;
    let mut all_safe = true;

    println!(
        "{} {}",
        c.action.paint("::"),
        c.bold.paint(tr!("Scanning PKGBUILDs for security issues..."))
    );

    for (pkg_name, pkg_dir) in packages {
        let result = scan_pkgbuild_dir(config, pkg_name, pkg_dir)?;

        print_scan_result(config, &result);

        if !result.is_safe {
            all_safe = false;
        }
    }

    println!();

    if all_safe {
        println!(
            "{} {}",
            c.action.paint("::"),
            Style::new().fg(ansiterm::Color::Green).paint(tr!("All packages passed security scan"))
        );
    } else {
        eprintln!(
            "{} {}",
            c.error.paint("::"),
            c.bold.paint(tr!("Some packages have security warnings - review carefully"))
        );
    }

    Ok(all_safe)
}

/// Print scan result for a single package.
fn print_scan_result(config: &Config, result: &SecurityScanResult) {
    let c = config.color;

    let tier_color = match result.tier.as_str() {
        "Trusted" | "Ok" => Style::new().fg(ansiterm::Color::Green),
        "Sketchy" => c.warning,
        "Suspicious" | "Malicious" => c.error,
        _ => Style::new(),
    };

    println!(
        "\n{} {} - {} (score: {:.1})",
        c.action.paint("::"),
        c.bold.paint(&result.package_name),
        tier_color.paint(&result.tier),
        result.score
    );

    if !result.signals.is_empty() {
        for signal in &result.signals {
            let signal_color = match signal.points {
                p if p >= 80.0 => c.error,
                p if p >= 50.0 => c.warning,
                _ => Style::new(),
            };

            println!(
                "    {} {}: {} ({:.0} pts)",
                c.bold.paint(&signal.category),
                signal_color.paint(&signal.id),
                signal.description,
                signal.points
            );
        }
    } else if result.tier == "Trusted" || result.tier == "Ok" {
        println!(
            "    {}",
            Style::new().fg(ansiterm::Color::Green).paint(tr!("No security issues detected"))
        );
    }
}
