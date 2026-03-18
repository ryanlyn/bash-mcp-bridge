# bash-mcp-bridge

A Rust MCP server that lets sandboxed agents execute host CLI commands against a binary name whitelist.

## Problem

Sandboxed environments like Claude Code Cowork run in isolated VMs that can't access host-side CLI tools. Even in non-sandboxed environments, there's value in restricting which binaries an agent can invoke. bash-mcp-bridge bridges this gap with a single MCP tool that validates commands against a whitelist before executing them.

## Design

### Tool

One MCP tool: `execute(command: string)` returns `{stdout, stderr, exit_code}`.

### Safety

1. Tokenize the command string into argv (shell-style tokenization, respecting quotes)
2. Parse into argv first, then reject standalone shell operator tokens such as `|`, `;`, `&&`, `||`, `>`, `<`, `&`
3. Resolve argv[0] via the host's PATH
4. Check if the binary name matches an entry in `allowed.bins`
5. Execute via subprocess (no shell), return stdout, stderr, and exit code

### Config

Single TOML file:

```toml
[server]
host = "127.0.0.1"
port = 8741
timeout = 120  # seconds

[allowed]
bins = ["gog", "uv"]
```

- `allowed.bins` is a list of binary names (not absolute paths)
- Binary resolution uses the host's PATH - whichever the PATH resolves first wins
- Config file is watched for changes; whitelist updates take effect on next `execute` call
- CLI `--allow` overrides take precedence over file-backed `allowed.bins`, including after reloads

### Transport

- stdio for local Claude Code CLI usage
- SSE/HTTP for Cowork connecting over the network

### Error handling

- **Rejected**: binary not in whitelist or standalone shell operator tokens detected. Returns a clear error message.
- **Execution failure**: binary not found on PATH, permission denied, timeout. Returns the OS-level error.
- **Non-zero exit**: not a server error. Returns `{stdout, stderr, exit_code}` as-is in structured tool output.

### Non-goals

- Argument-level validation or template interpolation
- Pipes, chaining, or multi-command execution
- Stateful sessions or operation composition
- Output transformation
- Authentication (localhost-only)
