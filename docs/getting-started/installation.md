# Installing Wormhole

Wormhole can be installed on Linux (Ubuntu, Debian, Fedora, Arch, NixOS) and Windows. Below are the main supported methods.

## 1. Windows

- Download `WormholeInstaller.exe` from the [GitHub Releases](https://github.com/Agartha-Software/Wormhole/releases)
- Run the installer and follow the instructions.

## 2. Ubuntu / Debian

- Download the latest `.deb` from the [GitHub Releases](https://github.com/Agartha-Software/Wormhole/releases)
- Install with:

```sh
sudo dpkg -i wormhole.deb
```

This will install both `wormhole` and `wormholed`.

## 3. Arch Linux (AUR)

You can use the AUR. Example with yay:

```sh
yay -S wormhole
```

This will install both `wormhole` and `wormholed`.

## 4. Fedora

- Download the latest `.rpm` from the [GitHub Releases](https://github.com/Agartha-Software/Wormhole/releases)
- Install with:

```sh
sudo dnf install ./wormhole.rpm
```

> [!WARNING]
> The package will give you access to `wormhole` and `wormholed` commands. `wormholed` is the service, but the package does not automatically enable it. You have to start it yourself

## 5. Nix / NixOS

This repository provides a flake for you that can install Wormhole.

### To try out wormhole

```sh
nix shell --experimental-features 'nix-command flakes' github:Agartha-Software/Wormhole/#default
```

You will then have access to Wormhole in the current shell.

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

You can then rebuild using `nixos-rebuild switch` and should have access to both `wormhole` and `wormholed`.

## 6. Install via Cargo or crates.io (All Platforms)

> [!WARNING]
> Installation via Cargo is not yet stable. You have to install fuse3 separately. If you do not success to install Wormhole using Cargo, use one of the other provided methods.

> [!WARNING]
> More rarely up to date. Please use the method from Github

```sh
cargo install wormhole-fs
```

This will install both `wormhole` and `wormholed`.

## 7. Build from source

**Requirements:**

- Install the [Rust toolchain](https://www.rust-lang.org/tools/install).
- Install fuse and other dependancies must be installed on your system:
  - **Linux:** `sudo apt install pkg-config libfuse3-dev libfuse-dev`  
  (Debian/Ubuntu) or equivalent for your distro
  - **Windows:** [WinFsp](https://github.com/winfsp/winfsp/releases)
- Clone and build:
```sh
git clone https://github.com/Agartha-Software/Wormhole.git
cd Wormhole
cargo build --release
```

Binaries will be in `target/release/` (`wormhole`, `wormholed`).

## Need Help?

- See the [Getting Started Guide](./getting_started.md)
- For Docker, see [Docker Guide](./docker_guide.md)
- For troubleshooting, open an issue on [GitHub](https://github.com/Agartha-Software/Wormhole/issues)
