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

## Deployment

### First-time install (on the target server)

```bash
# Clone repo (just for the deploy scripts + example config)
git clone https://github.com/holoujak/fuel-logger-rs.git /tmp/fuel-logger-rs
cd /tmp/fuel-logger-rs

# Run install script (creates user, downloads latest binary, installs systemd service)
sudo ./deploy/install.sh

# Edit the config
sudo nano /opt/fuel-logger-rs/config.toml

# Start!
sudo systemctl start fuel-logger-rs
```

### Updating to a new version

```bash
# Update to latest release
sudo /opt/fuel-logger-rs/update.sh

# Or update to a specific version
sudo /opt/fuel-logger-rs/update.sh v1.2.0
```

The update script automatically:
- Downloads the correct binary for your architecture (x86_64/aarch64)
- Stops the service, swaps the binary, restarts
- Rolls back if the new version fails to start

### Creating a release

Push a tag to trigger the CI release build:

```bash
git tag v1.0.0
git push origin v1.0.0
```

Then create a release on GitHub from that tag — the CI will automatically build and attach binaries for both x86_64 and aarch64.

### Useful commands

```bash
sudo systemctl status fuel-logger-rs    # Check status
sudo journalctl -u fuel-logger-rs -f    # Follow logs
sudo systemctl restart fuel-logger-rs   # Restart
```
