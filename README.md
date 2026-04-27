<div align="center">
  <img width="200" height="auto" src="docs/v2/public/images/logo.svg" alt="Infrarust Logo">

  <h1>SpacerCord</h1>

  <p>High-performance Minecraft reverse proxy with SpacetimeDB integration. A specialized fork of <a href="https://github.com/Shadowner/Infrarust">Infrarust</a> designed for persistent game state and advanced networking.</p>
  <a href="https://ko-fi.com/C1C41WOEBB">
    <img height='26' alt="Ko-fi" src="https://storage.ko-fi.com/cdn/kofi6.png?v=6" />
  <br /><br />
  </a>
  <img alt="License" src="https://img.shields.io/badge/license-AGPL--3.0-blue?style=flat-square" />
  <a href="https://discord.gg/sqbJhZVSgG">
    <img alt="Discord" src="https://img.shields.io/discord/1330603066478825502?style=flat-square&label=discord" />
  </a>

</div>

<br />

<p align="center">
  <img src="docs/v2/public/web-ui.png" alt="SpacerCord web dashboard" width="800" />
</p>

> [!WARNING]
> SpacerCord is currently in active development. Expect bugs with intercepted modes (client_only / offline).

## Features

| | |
|---|---|
| **SpacetimeDB** | **Built-in database SDK** for persisting game state, player data, and cross-server logic directly in the proxy. |
| **Routing** | Domain and subdomain-based routing with wildcard support. One port, many servers. |
| **Proxy modes** | `passthrough`, `zerocopy`, `client_only`, `offline`, `server_only` - from raw TCP relay to full Mojang auth interception. |
| **Web dashboard** | Built-in admin panel with REST API, real-time event streaming (SSE), and log viewer. |
| **Docker** | Auto-discover Minecraft containers via labels - no config files needed for Docker-managed servers. |
| **Plugins** | 3-tier event-driven plugin system. Intercept packets or host virtual mini-games entirely inside the proxy. |
| **Security** | Rate limiting, IP filtering, ban system (IP / UUID / username). |
| **Observability** | OpenTelemetry export for metrics, traces, and logs. |
| **Hot reload** | Drop a `.toml` file in `servers/` and the proxy picks it up. No restart. |

## SpacetimeDB Integration

SpacerCord bundles the **SpacetimeDB SDK**, allowing you to persist player data, world state, and proxy-wide events directly within the proxy's runtime.

- **Persistent State:** Store player logins, play time, and cross-server data without external databases.
- **Async Driver:** High-performance MPSC bridge ensures SpacetimeDB operations never block the networking hot path.
- **Relational Logic:** Use SpacetimeDB's relational model to query and manipulate proxy state with ease.

## Quick Start

### Install

```bash
# From source (Rust 1.94+)
git clone https://github.com/Shadowner/SpacerCord.git && cd SpacerCord
cargo build --release -p infrarust
```

### Configure

`infrarust.toml`:

```toml
bind = "0.0.0.0:25565"
servers_dir = "./servers"

[web]
```

`servers/survival.toml`:

```toml
domains = ["survival.example.com"]
addresses = ["127.0.0.1:25566"]
```

### Run

```bash
./target/release/infrarust
```

The web dashboard is at `http://localhost:8080`. Your API key is in `plugins/admin_api/config.toml`.

## Documentation

Full documentation at **[docs.spacercord.com](https://docs.spacercord.com)** (Coming Soon).

## Contributing

Contributions welcome - see [CONTRIBUTING.md](CONTRIBUTING.md) for setup and guidelines.

Questions or ideas? Join the [Discord](https://discord.gg/sqbJhZVSgG) or open an issue.

## License

AGPL-3.0 with plugin exceptions - see [LICENSE](LICENSE).

<p align="center">
  <img height="60" src="docs/v1/public/img/agplv3_logo.svg" alt="AGPL v3" />
</p>
