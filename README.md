# fuel-logger-rs
Rust application to control small petrol pump from RPi

## Prerequisites

### nix

Install nix
```bash
$ sh <(curl --proto '=https' --tlsv1.2 -L https://nixos.org/nix/install) --daemon
```

Update your user config
```bash
$ mkdir -p ~/.config/nix
$ echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

Get a development bash shell with dependencies in PATH:
`nix develop`
