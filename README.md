# bash-mcp-bridge

MCP server that lets sandboxed agents execute host CLI commands against a binary whitelist.

Commands are executed directly without a shell. Standalone shell operator tokens (`|`, `;`, `&&`, `||`, `>`, `<`, `&`) are rejected. Successful calls return structured `stdout`, `stderr`, and `exit_code` data, including for non-zero exit codes.

## Install

```bash
cargo install --git https://github.com/ryanlyn/bash-mcp-bridge
```

Or build from source:

```bash
git clone https://github.com/ryanlyn/bash-mcp-bridge
cd bash-mcp-bridge
cargo build --release
```

## Usage

### Claude Cowork

Add to your MCP config (`.mcp.json`):

```json
{
  "mcpServers": {
    "bash-bridge": {
      "command": "bash-mcp-bridge",
      "args": ["--allow", "gog"],
      "env": {
        "XDG_CONFIG_HOME": "<env vars are not inherited>"
      }
    }
  }
}
```

### Options

| Flag | Description |
|------|-------------|
| `-c, --config <path>` | Path to TOML config file (optional if `--allow` is provided) |
| `-t, --transport <type>` | `stdio` (default) or `http` |
| `--allow <binary>` | Allow a binary by name (repeatable, overrides config file whitelist) |

## Config

```toml
[server]
host = "127.0.0.1"  # HTTP transport bind address
port = 8741          # HTTP transport port
timeout = 120        # command timeout in seconds

[allowed]
bins = ["gog"]
```

All fields in `[server]` are optional with the defaults shown above. The config file is watched for changes - whitelist updates take effect on the next `execute` call. CLI `--allow` flags take precedence over the config file, including after reloads.

## Tools

### `list_allowed`

Lists the binaries that are allowed and usage hints for each.

### `execute`

Executes a command on the host.

```
execute(command: "gog --help")
execute(command: "gog gmail --help")
```

## Safety

- **No shell**: commands run via direct subprocess execution, not through a shell. Shell expansion, globbing, and variable substitution do not apply.
- **Binary whitelist**: only binaries explicitly listed in `allowed.bins` or `--allow` flags can be invoked.
- **Token rejection**: standalone shell operator tokens (`|`, `;`, `&&`, `||`, `>`, `<`, `&`) are rejected before execution. Quoted literals containing these characters are fine.
- **Timeouts**: commands that exceed the configured timeout are killed.
- **Localhost-only HTTP**: the HTTP transport binds to `127.0.0.1` by default.

## License

MIT
