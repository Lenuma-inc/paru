# Paru

Feature packed AUR helper with integrated security scanning

> **Note**: This is a modified fork of Paru for **LunaOS**.
> Fork-specific features are listed below in **Fork Differences (LunaOS)**.
> Upstream: https://github.com/Morganamilo/paru
> This fork: https://github.com/Lenuma-inc/paru
> Issue tracker for this fork: https://github.com/Lenuma-inc/paru/issues

[![paru](https://img.shields.io/aur/version/paru?color=1793d1&label=paru&logo=arch-linux&style=for-the-badge)](https://aur.archlinux.org/packages/paru/)
[![paru-bin](https://img.shields.io/aur/version/paru-bin?color=1793d1&label=paru-bin&logo=arch-linux&style=for-the-badge)](https://aur.archlinux.org/packages/paru-bin/)
[![paru-git](https://img.shields.io/aur/version/paru-git?color=1793d1&label=paru-git&logo=arch-linux&style=for-the-badge)](https://aur.archlinux.org/packages/paru-git/)

## Description

Paru is your standard pacman wrapping AUR helper with lots of features and minimal interaction.

## Fork Differences (LunaOS)

Compared to upstream Paru, this fork includes:

- Integrated PKGBUILD security scanning before build/install.
- Built-in `paru downgrade` subcommand (ALA-based rollback and package downgrade workflow).
- Strict typo tolerance for repository package names in interactive install search (`paru <target>`), applied only to repository results (not AUR) and intentionally strict to avoid incorrect matches and unnecessary slowdown.
- `CombinedUpgrade` and `UpgradeMenu` are enabled by default, so `paru` shows a unified repo+AUR update menu in a `yay`-like format before installation.
- Third-party attributions and licenses: [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md).

[![asciicast](https://asciinema.org/a/sEh1ZpZZUgXUsgqKxuDdhpdEE.svg)](https://asciinema.org/a/sEh1ZpZZUgXUsgqKxuDdhpdEE)

## Installation

```
sudo pacman -S --needed base-devel
git clone https://github.com/Lenuma-inc/paru.git
cd paru
makepkg -si
```

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## General Tips

- **Man pages**: For documentation on paru's options and config file see `paru(8)` and `paru.conf(5)` respectively.

- **Color**: Paru only enables color if color is enabled in pacman. Enable `color` in your `pacman.conf`.

- **Security scanning**: Paru automatically scans PKGBUILDs for security issues before building. Pay attention to security warnings and review flagged packages carefully.

- **Flip search order**: To get search results to start at the bottom and go upwards, enable `BottomUp` in `paru.conf`.

- **Tracking -git packages**: Paru tracks -git package by monitoring the upstream repository. Paru can only do this for packages that paru itself installed. `paru --gendb` will make paru aware of packages it did not install.

- **PKGBUILD syntax highlighting**: You can install [`bat`](https://github.com/sharkdp/bat) to enable syntax highlighting when viewing PKGBUILDs with `-G --print`.

## Examples

`paru <target>` -- Interactively search and install `<target>`.

`paru` -- Alias for `paru -Syu`.

`paru -S <target>` -- Install a specific package.

`paru -Sua` -- Upgrade AUR packages.

`paru -Qua` -- Print available AUR updates.

`paru -G <target>` -- Download the PKGBUILD and related files of `<target>`.

`paru -Gp <target>` -- Print the PKGBUILD of `<target>`.

`paru -Gc <target>` -- Print the AUR comments  of `<target>`.

`paru --gendb` -- Generate the devel database for tracking `*-git` packages. This is only needed when you initially start using paru.

`paru -Bi .` -- Build and install a PKGBUILD in the current directory.

`paru downgrade <pkg>` -- Show archived versions from ALA and downgrade a package interactively.

`paru downgrade <pkg> --date 2024-12-01` -- Downgrade a package to the latest version available on/before a date.

`paru downgrade --date 2024-12-01` -- Roll back repository packages/system to an ALA snapshot date.

## IRC

Paru now has an IRC. #paru on [Libera Chat](https://libera.chat/). Feel free to join for discussion and help with paru.

## Debugging

Paru is not an official tool. If paru can't build a package, you should first check if makepkg can successfully build the package. If it can't, then you should report the issue to the maintainer. Otherwise, it is likely an issue with paru and should be reported here.
