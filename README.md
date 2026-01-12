<div align="center" style="text-align:center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="images/logo_text_dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="images/logo_text_light.svg">
    <img alt="ncspot logo" height="128" src="images/logo_text_light.svg">
  </picture>
  <h3>An ncurses Spotify client written in Rust using librespot</h3>

[![Crates.io](https://img.shields.io/crates/v/ncspot.svg)](https://crates.io/crates/ncspot)
[![Gitter](https://badges.gitter.im/ncspot/community.svg)](https://gitter.im/ncspot/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

  <img alt="ncspot search tab" src="images/screenshot.png">
</div>

ncspot is an ncurses Spotify client written in Rust using librespot. It is heavily inspired by
ncurses MPD clients, such as [ncmpc](https://musicpd.org/clients/ncmpc/). My motivation was to
provide a simple and resource friendly alternative to the official client as well as to support
platforms that currently don't have a Spotify client, such as the \*BSDs.

ncspot only works with a Spotify premium account as it offers features that are not available for
free accounts.

## Features
- Support for tracks, albums, playlists, genres, searching...
- Small [resource footprint](doc/resource_footprint.md)
- Support for a lot of platforms
- Vim keybindings out of the box
- IPC socket for remote control

## Authentication

ncspot requires you to create your own Spotify Developer application for OAuth2 authentication. This provides better privacy and allows you to control your own credentials.

### First-Time Setup

When you run ncspot for the first time, you'll be guided through an interactive setup wizard:

1. Go to https://developer.spotify.com/dashboard/applications
2. Click "Create app" and fill in a name and description
3. Add `http://127.0.0.1:8888/callback` to Redirect URIs (or use a custom port)
4. Save your app and copy the Client ID and Client Secret
5. Enter the credentials when prompted by ncspot

Your credentials will be saved to `~/.config/ncspot/client.yml`.

### Client Configuration

The `client.yml` file supports the following options:

```yaml
client_id: "your_32_character_client_id"
client_secret: "your_32_character_client_secret"
port: 8888  # optional, default port for OAuth redirect
```

> **Note:** Make sure the redirect URI in your Spotify Dashboard matches `http://127.0.0.1:<port>/callback`

### Token Caching

After successful authentication, ncspot caches your access token and automatically refreshes it when needed. You won't need to re-authenticate unless you explicitly delete the cached token at `~/.config/ncspot/.spotify_token_cache.json`.

## Installation
ncspot is available on macOS (Homebrew), Windows (Scoop, WinGet), Linux (native package, Flathub and
Snapcraft) and the BSD's. Detailed installation instructions for each platform can be found
[here](/doc/users.md).

## Configuration
A configuration file can be provided. The default location is `~/.config/ncspot`. Detailed
configuration information can be found [here](/doc/users.md#configuration).

ncspot uses two configuration files:
- `config.toml` - General application settings (theme, keybindings, etc.)
- `client.yml` - Spotify OAuth credentials (see [Authentication](#authentication))

## Building
Building ncspot requires a working [Rust installation](https://www.rust-lang.org/tools/install) and
a Python 3 installation. To compile ncspot, run `cargo build`. For detailed instructions on building
ncspot, there is more information [here](/doc/developers.md).

## Packaging
Information about provided files, how to generate some of them and current package status accross
platforms can be found [here](/doc/package_maintainers.md).
