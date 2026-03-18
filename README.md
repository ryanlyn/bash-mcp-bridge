# bash-bridge-mcp

MCP server that lets sandboxed agents execute host CLI commands against a binary name whitelist.

## Usage

```bash
# stdio transport (for Claude Code CLI)
bash-bridge-mcp -c config.toml -t stdio

# HTTP transport (for Cowork)
bash-bridge-mcp -c config.toml -t http
```

## Config

```toml
[server]
host = "127.0.0.1"
port = 8741
timeout = 120

[allowed]
bins = ["gog", "uv"]
```

See [DESIGN.md](DESIGN.md) for full details.
