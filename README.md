# bash-mcp-bridge

MCP server that lets sandboxed agents execute host CLI commands against a binary name whitelist.

Commands are executed directly without a shell. Successful calls return structured
`stdout`, `stderr`, and `exit_code` data, including non-zero exit codes.

## Usage

```bash
# stdio transport (for Claude Code CLI)
bash-mcp-bridge -c config.toml -t stdio

# HTTP transport (for Cowork)
bash-mcp-bridge -c config.toml -t http
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
