//! Pattern-driven PKGBUILD security scanner.

use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Trusted,
    Ok,
    Sketchy,
    Suspicious,
    Malicious,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub id: String,
    pub description: String,
    pub points: f64,
    pub category: String,
    pub override_gate: bool,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub tier: Tier,
    pub score: f64,
    pub signals: Vec<Signal>,
}

#[derive(Debug, Deserialize)]
struct PatternRule {
    id: String,
    pattern: String,
    points: f64,
    description: String,
    #[serde(default)]
    override_gate: bool,
}

#[derive(Debug, Default, Deserialize)]
struct PatternDb {
    #[serde(default)]
    pkgbuild_analysis: Vec<PatternRule>,
    #[serde(default)]
    source_url_analysis: Vec<PatternRule>,
    #[serde(default)]
    gtfobins_analysis: Vec<PatternRule>,
}

#[derive(Debug)]
struct CompiledRule {
    id: String,
    regex: Regex,
    points: f64,
    description: String,
    category: &'static str,
    override_gate: bool,
}

static RULES: OnceLock<Vec<CompiledRule>> = OnceLock::new();

fn rules() -> &'static [CompiledRule] {
    RULES.get_or_init(load_rules).as_slice()
}

fn load_rules() -> Vec<CompiledRule> {
    let raw = include_str!("../security/patterns.toml");
    let db: PatternDb = match toml::from_str(raw) {
        Ok(db) => db,
        Err(err) => {
            // Fail closed: parser errors must not silently disable security rules.
            return vec![scanner_error_rule(format!(
                "failed to parse pattern database: {err}"
            ))];
        }
    };
    let mut out = Vec::new();
    let mut invalid_regexes = Vec::new();
    append_rules(
        &mut out,
        &mut invalid_regexes,
        "Pkgbuild",
        &db.pkgbuild_analysis,
    );
    append_rules(
        &mut out,
        &mut invalid_regexes,
        "SourceUrl",
        &db.source_url_analysis,
    );
    append_rules(
        &mut out,
        &mut invalid_regexes,
        "GTFOBins",
        &db.gtfobins_analysis,
    );
    if !invalid_regexes.is_empty() {
        // Fail closed: invalid regexes indicate scanner misconfiguration.
        return vec![scanner_error_rule(format!(
            "invalid regex patterns: {}",
            invalid_regexes.join(", ")
        ))];
    }
    out
}

fn append_rules(
    out: &mut Vec<CompiledRule>,
    invalid_regexes: &mut Vec<String>,
    category: &'static str,
    rules: &[PatternRule],
) {
    for r in rules {
        match Regex::new(&r.pattern) {
            Ok(regex) => out.push(CompiledRule {
                id: r.id.clone(),
                regex,
                points: r.points,
                description: r.description.clone(),
                category,
                override_gate: r.override_gate,
            }),
            Err(err) => invalid_regexes.push(format!("{} ({err})", r.id)),
        }
    }
}

fn scanner_error_rule(reason: String) -> CompiledRule {
    // Always match to force a visible hard warning and stop unsafe installs.
    let regex = Regex::new("(?s).*").expect("constant regex must compile");
    CompiledRule {
        id: "P-SCANNER-ERROR".to_string(),
        regex,
        points: 100.0,
        description: format!("Security scanner configuration error: {reason}"),
        category: "Scanner",
        override_gate: true,
    }
}

pub fn scan_pkgbuild(_pkg_name: &str, pkgbuild_content: &str) -> ScanResult {
    let mut signals = Vec::new();

    for rule in rules() {
        if rule.regex.is_match(pkgbuild_content) {
            signals.push(Signal {
                id: rule.id.clone(),
                description: rule.description.clone(),
                points: rule.points,
                category: rule.category.to_string(),
                override_gate: rule.override_gate,
            });
        }
    }

    // Keep checksum coverage even if pattern db is changed.
    let has_checksums = pkgbuild_content.lines().any(|line| {
        line.contains("sha512sums=")
            || line.contains("sha384sums=")
            || line.contains("sha256sums=")
            || line.contains("sha224sums=")
            || line.contains("sha1sums=")
            || line.contains("md5sums=")
            || line.contains("b2sums=")
    });
    if !has_checksums {
        signals.push(Signal {
            id: "P-MISSING-CHECKSUM".to_string(),
            description: "No checksum array found in PKGBUILD".to_string(),
            points: 40.0,
            category: "Integrity".to_string(),
            override_gate: false,
        });
    }

    let score: f64 = signals.iter().map(|s| s.points).sum();
    let score = if score.abs() < f64::EPSILON { 0.0 } else { score };
    let tier = if score >= 80.0 {
        Tier::Malicious
    } else if score >= 60.0 {
        Tier::Suspicious
    } else if score >= 30.0 {
        Tier::Sketchy
    } else if score > 0.0 {
        Tier::Ok
    } else {
        Tier::Trusted
    };

    ScanResult {
        tier,
        score,
        signals,
    }
}
