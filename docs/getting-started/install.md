# Installing Wormhole

Wormhole can be installed on Linux (Ubuntu, Debian, Arch, NixOS) and Windows. Below are the main supported methods.

---

## 1. Arch Linux (AUR)

You can use the AUR. Example with yay:

```sh
yay -S wormhole
```

This will install both `wormhole` and `wormholed` binaries.

---

## 2. Nix / NixOS

This repository provides a flake for you that can install Wormhole.

### To try

```sh
nix shell --experimental-features 'nix-command flakes' github:Agartha-Software/Wormhole/#default
```

You will then get Wormhole on this ephemeral shell.

### To install

Add Wormhole in your flake inputs:

```nix
# flake.nix
inputs = {
  ...
  wormhole.url = "github:Agartha-Software/Wormhole"; # add this in your inputs
  ...
};
```

Add the package in your configuration:

```nix
# configuration.nix
environment.systemPackages = with pkgs; [
  ...
  inputs.wormhole.packages.${pkgs.system}.wormhole # wormhole package
  ...
];
```

Add the systemd service if needed:

```nix
# flake.nix
modules = [
  ...
  inputs.wormhole.nixosModules.wormhole
    {
      services.wormhole.enable = true;
    }
  ...
]
```

You can then rebuild using `nixos-rebuild switch` and should have access to both `wormhole` and `wormholed` binaries.

---

## 3. Install via Cargo or crates.io (All Platforms)

**Requirements:**

- [Rust toolchain](https://www.rust-lang.org/tools/install)
- FUSE must be installed on your system:
  - **Linux:** `sudo apt install libfuse3-dev` (Debian/Ubuntu) or equivalent for your distro
  - **Windows:** [WinFsp](https://github.com/winfsp/winfsp/releases)
  - **NixOS:** FUSE is managed by the package manager

### a) From crates.io

```sh
cargo install wormhole-fs
```

This will install both `wormhole` and `wormholed` binaries.

### b) From GitHub (latest features)

```sh
git clone https://github.com/Agartha-Software/Wormhole.git
cd Wormhole
cargo build --release
```

Binaries will be in `target/release/` (`wormhole`, `wormholed`).

---

## 4. Ubuntu / Debian

### a) Using the .deb package (recommended)

- Download the latest `.deb` from the [GitHub Releases](https://github.com/Agartha-Software/Wormhole/releases)
- Install with:

```sh
sudo dpkg -i wormhole.deb
```

This will install both `wormhole` and `wormholed`.

### b) Manual build (if you want the latest or custom build)

- Install dependencies:

```sh
sudo apt update
sudo apt install -y pkg-config libfuse3-dev libfuse-dev
```

- Then follow the Cargo instructions above.

---

## 5. Windows

### a) Using the Installer

- Download `WormholeInstaller.exe` from the [GitHub Releases](https://github.com/Agartha-Software/Wormhole/releases)
- Run the installer and follow the instructions.

### b) Manual build (advanced)

- Install [Rust](https://www.rust-lang.org/tools/install)
- Install [WinFsp](https://github.com/winfsp/winfsp/releases)
- Clone and build:

```powershell
git clone https://github.com/Agartha-Software/Wormhole.git
cd Wormhole
cargo build --release
```

Binaries will be in `target\release\` (`wormhole.exe`, `wormholed.exe`).

---

## Need Help?

- See the [Getting Started Guide](./getting_started.md)
- For Docker, see [Docker Guide](./docker_guide.md)
- For CLI usage, see [CLI Memo](./memo_cli.md)
- For troubleshooting, open an issue on [GitHub](https://github.com/Agartha-Software/Wormhole/issues)