# Getting Started with spn-cli

**Get productive with spn in 5 minutes.** This guide will walk you through installation, configuration, and your first workflow execution.

---

## What You'll Learn

By the end of this tutorial, you'll be able to:

- Install spn on your system
- Configure LLM providers securely
- Add and use MCP servers
- Sync packages to your AI editor
- Run your first command successfully

**Time Required:** 5 minutes
**Prerequisites:** macOS or Linux (Windows support coming soon)

---

## 1. Installation (30 seconds)

Choose your preferred installation method:

### Option A: Homebrew (Recommended for macOS)

```bash
brew install supernovae-st/tap/spn
```

### Option B: Cargo (Cross-platform)

```bash
cargo install spn-cli
```

### Option C: Docker (For containers)

```bash
docker pull ghcr.io/supernovae-st/spn:latest
```

> **Note:** Docker cannot access OS Keychain. Use environment variables for API keys in containers.

### Verify Installation

```bash
spn --version
# Expected output: spn-cli 0.15.x
```

**First run diagnostic:**

```bash
spn doctor
```

You should see a health check that verifies:
- spn installation ✓
- Dependencies (Node.js, npm, Git)
- Configuration files
- System compatibility

> **Troubleshooting:** If `spn` command is not found, ensure the installation directory is in your `$PATH`:
> - Homebrew: `/opt/homebrew/bin` (Apple Silicon) or `/usr/local/bin` (Intel)
> - Cargo: `~/.cargo/bin`

---

## 2. First Run: Setup Wizard (1 minute)

The setup wizard will guide you through initial configuration:

```bash
spn setup
```

**What happens during setup:**

1. **Welcome Screen** — Explains what spn does (Package Manager + Secrets Manager + Sync Manager)
2. **Ecosystem Tools Check** — Detects if Nika/NovaNet are installed (optional)
3. **Key Detection** — Scans environment variables for existing API keys
4. **Provider Selection** — Choose which LLM providers to configure
5. **Migration Prompt** — Offers to migrate keys to OS Keychain (recommended)

**Example interaction:**

```
🌟 SuperNovae Setup Wizard

WHAT IS SPN?

  📦 Package Manager
     Install AI workflows, schemas, skills, and MCP servers

  🔐 Secrets Manager
     Securely store API keys for LLM providers and MCP tools

  🔄 Sync Manager
     Sync packages to Claude Code, VS Code, and other editors

Ready to set up spn? [Y/n]
```

**Quick setup mode (skip explanations):**

```bash
spn setup --quick
```

---

## 3. Configure Providers (2 minutes)

### Understanding Providers

spn supports **7 LLM providers** and **6 MCP secrets**:

| Provider | Signup URL | Cost | Best For |
|----------|-----------|------|----------|
| **🦙 Ollama** | [ollama.ai](https://ollama.ai) | **Free** | Local inference, full privacy |
| Anthropic (Claude) | [console.anthropic.com](https://console.anthropic.com/settings/keys) | Paid | Complex reasoning, coding |
| OpenAI (GPT-4) | [platform.openai.com](https://platform.openai.com/api-keys) | Paid | General purpose, vision |
| Gemini | [aistudio.google.com](https://aistudio.google.com/app/apikey) | **Free tier** | Multimodal |
| Groq | [console.groq.com](https://console.groq.com/keys) | **Free tier** | Ultra-fast inference |
| Mistral | [console.mistral.ai](https://console.mistral.ai/api-keys) | Paid | European, code generation |
| DeepSeek | [platform.deepseek.com](https://platform.deepseek.com/api_keys) | **Free tier** | Cost-effective reasoning |

> **💡 Tip:** Start with **Ollama** for local development — no API keys, no costs, full privacy.

### Add Your First API Key

Let's add an Anthropic API key as an example:

```bash
spn provider set anthropic
```

**Interactive prompt:**

```
🔐 Setting API key for: anthropic

Enter API key (input hidden):
Confirm API key:

✅ Key stored in OS Keychain
   Security: Encrypted at rest
   Location: macOS Keychain

Test connection: spn provider test anthropic
```

**Why OS Keychain?**

spn stores API keys in your operating system's secure keychain:
- **macOS:** Keychain Access (encrypted, protected by login)
- **Linux:** Secret Service (gnome-keyring, KWallet)
- **Windows:** Credential Manager

**Benefits:**
- Encrypted at rest by the OS
- No `.env` files to accidentally commit
- Single source of truth across all tools
- Memory protection with `mlock()` and auto-zeroization

### Test Your API Key

```bash
spn provider test anthropic
```

**Expected output:**

```
🧪 Testing: anthropic

  Format: ✅ Valid (sk-ant-api03-...)
  Length: ✅ Correct (64 characters)
  Prefix: ✅ Valid (sk-ant-api03-)

Key is valid and ready to use!
```

### View Configured Providers

```bash
spn provider list
```

**Output:**

```
🔐 Stored API Keys

  anthropic: sk-ant-***************X (OS Keychain) ✓
  openai:    sk-***************X (Environment) ⚠️

Legend:
  ✓ = Secure (OS Keychain)
  ⚠️ = Less secure (env var or .env file)

Migrate to keychain: spn provider migrate
```

### Optional: Migrate Environment Variables

If you have API keys in `.env` files or environment variables:

```bash
spn provider migrate
```

This will:
1. Scan for known environment variables (e.g., `ANTHROPIC_API_KEY`)
2. Show what will be migrated
3. Store each key in OS Keychain
4. Remind you to remove from `.env` files

---

## 4. Add MCP Servers (1 minute)

**What are MCP servers?** MCP (Model Context Protocol) servers provide tools that LLMs can use — like database access, web search, file operations, etc.

### Built-in Aliases

spn includes **48 pre-configured MCP server aliases**. Add them by name:

```bash
# Add Neo4j graph database
spn mcp add neo4j

# Add GitHub integration
spn mcp add github

# Add web search
spn mcp add perplexity
```

**What happens:**

1. Resolves alias to npm package (e.g., `neo4j` → `@neo4j/mcp-server-neo4j`)
2. Installs npm package globally
3. Adds to team config (`./mcp.yaml`)
4. Auto-syncs to enabled editors

### View Installed MCP Servers

```bash
spn mcp list
```

**Output:**

```
🔌 Installed MCP Servers

Team Servers (from ./mcp.yaml):
  neo4j     @neo4j/mcp-server-neo4j v0.1.0
  github    @modelcontextprotocol/server-github v0.2.0

Total: 2 servers

Test: spn mcp test <name>
```

### Test MCP Server Connection

```bash
spn mcp test neo4j
```

**Expected output:**

```
🧪 Testing: neo4j

  Package: @neo4j/mcp-server-neo4j v0.1.0
  Command: npx -y @neo4j/mcp-server-neo4j

  Environment:
    NEO4J_URI:      bolt://localhost:7687 ✓
    NEO4J_PASSWORD: ********** ✓

  Connection: ✅ Server responds
  Tools: 8 available
    - neo4j_query
    - neo4j_execute
    - neo4j_schema
    - ...

Server is healthy and ready to use!
```

### Popular MCP Servers

```bash
# Database
spn mcp add neo4j postgres sqlite

# Development
spn mcp add github gitlab filesystem

# Search & AI
spn mcp add perplexity brave-search

# Web Scraping
spn mcp add firecrawl puppeteer

# Communication
spn mcp add slack discord
```

Run `spn mcp list --all` to see all 48 aliases.

---

## 5. Sync to Editors (30 seconds)

**What gets synced?**

- MCP server configurations
- Skills (if you add any)
- Provider settings

### Enable Your Editor

```bash
# Enable Claude Code
spn sync --enable claude-code

# Enable Cursor
spn sync --enable cursor

# Enable Windsurf
spn sync --enable windsurf
```

### Sync Now

```bash
spn sync
```

**Interactive mode (preview changes):**

```bash
spn sync --interactive
```

**Output:**

```
🔄 Syncing to enabled editors...

📂 Claude Code (.claude/settings.json)
  + neo4j MCP server
  + github MCP server

📂 Cursor (.cursor/mcp.json)
  + neo4j MCP server
  + github MCP server

Apply changes? [Y/n]
```

### Verify in Claude Code

1. Open Claude Code
2. Open settings (Cmd+Shift+P → "Claude: Open Settings")
3. Verify MCP servers appear in the config

**Example `.claude/settings.json`:**

```json
{
  "mcpServers": {
    "neo4j": {
      "command": "npx",
      "args": ["-y", "@neo4j/mcp-server-neo4j"],
      "env": {
        "NEO4J_URI": "bolt://localhost:7687"
      }
    }
  }
}
```

> **🦋 Nika Note:** If you use Nika workflows, they read MCP configs **directly** from `~/.spn/mcp.yaml` — no sync needed!

---

## 6. Verify Everything Works

### Run Status Dashboard

```bash
spn status
```

**Expected output:**

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  ✦ spn status                                    The Agentic AI Toolkit  ✦  ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

┌─ 🔑 CREDENTIALS ─────────────────────────────────────────────────────────────┐
│  Name          Type   Status      Source      Endpoint                       │
│  anthropic     LLM    ✅ ready     🔐 keychain api.anthropic.com             │
│  neo4j         MCP    ✅ ready     🔐 keychain bolt://localhost:7687          │
│  2/13 configured   │   🔐 2 keychain                                          │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ 🔌 MCP SERVERS ─────────────────────────────────────────────────────────────┐
│  Server        Status      Transport   Command             Credential         │
│  neo4j         ○ ready     stdio       npx                 → neo4j            │
│  github        ○ ready     stdio       npx                 → github           │
│  2/2 active                                                                   │
└──────────────────────────────────────────────────────────────────────────────┘

  🔑 2/13 Keys    🔌 2/2 MCPs    📡 Daemon OK
```

**This dashboard shows:**
- Which API keys are configured and where they're stored
- Which MCP servers are installed and their status
- Overall system health

---

## What's Next?

You're now set up and ready to use spn! Here are some next steps:

### Explore Local Models

Run LLMs locally with Ollama (no API keys needed):

```bash
# Install Ollama from ollama.ai first, then:

# List available models
spn model list

# Download a model
spn model pull llama3.2:1b

# Load model into memory
spn model load llama3.2:1b

# Check status
spn model status
```

### Add Skills

Skills are reusable AI prompts that enhance your editor:

```bash
# Search for skills
spn skill search workflow

# Add a skill
spn skill add brainstorming

# List installed skills
spn skill list
```

Skills automatically sync to `.claude/skills/`, `.cursor/skills/`, etc.

### Install Nika (Workflow Engine)

Nika executes AI workflows defined in YAML:

```bash
spn setup nika
```

This will:
1. Install `nika` binary
2. Set up IDE integration
3. Configure the daemon
4. Test the installation

Then create workflows like:

```yaml
# example.yaml
workflow: generate-page
steps:
  - infer: "Generate a landing page about QR codes"
    use.ctx: page_content
  - exec: "echo $page_content > page.html"
```

Run with:

```bash
nika run example.yaml
```

### Install NovaNet (Knowledge Graph)

NovaNet provides localization and semantic context:

```bash
spn setup novanet
```

---

## Common Commands Cheat Sheet

```bash
# Status & Health
spn doctor              # System diagnostic
spn status              # Dashboard view

# Providers (API Keys)
spn provider set <name>    # Add key
spn provider list          # View all keys
spn provider test <name>   # Validate key
spn provider migrate       # Move to keychain

# MCP Servers
spn mcp add <name>      # Add server
spn mcp list            # View all servers
spn mcp test <name>     # Test connection

# Skills
spn skill add <name>    # Add skill
spn skill list          # View all skills

# Models (Local LLMs)
spn model pull <name>   # Download model
spn model list          # View models
spn model load <name>   # Load to RAM
spn model status        # Check VRAM usage

# Sync
spn sync                # Sync to editors
spn sync --status       # View sync status
```

---

## Troubleshooting

### "command not found: spn"

**Solution:** Add installation directory to `$PATH`:

```bash
# Homebrew (Apple Silicon)
echo 'export PATH="/opt/homebrew/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Cargo
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### "Failed to access keychain"

**Solution (macOS):** Reset keychain permissions:

```bash
security unlock-keychain ~/Library/Keychains/login.keychain-db
```

**Solution (Linux):** Ensure secret service is running:

```bash
systemctl --user status gnome-keyring-daemon
```

### "MCP server failed to start"

**Solution:** Check dependencies and credentials:

```bash
# Test specific server
spn mcp test neo4j

# Check npm package
npm list -g @neo4j/mcp-server-neo4j

# Verify API key
spn provider get neo4j
```

### "Sync has no effect"

**Solution:** Enable editor sync:

```bash
# Check status
spn sync --status

# Enable your editor
spn sync --enable claude-code

# Force sync
spn sync --interactive
```

---

## Getting Help

- **Documentation:** Full command reference in [README.md](../README.md)
- **GitHub Issues:** [supernovae-st/supernovae-cli/issues](https://github.com/supernovae-st/supernovae-cli/issues)
- **Discord:** [discord.gg/supernovae](https://discord.gg/supernovae)
- **Verbose Mode:** Add `-v`, `-vv`, or `-vvv` to any command for debug output

**Example:**

```bash
spn -vv provider set anthropic
```

---

## Success Checklist

After completing this tutorial, you should have:

- [ ] spn installed and verified with `spn --version`
- [ ] At least one LLM provider configured (e.g., Anthropic or Ollama)
- [ ] At least one MCP server added (e.g., neo4j or github)
- [ ] Editor sync enabled for Claude Code/Cursor/Windsurf
- [ ] Status dashboard shows healthy system (`spn status`)

**Congratulations!** You're now ready to build AI workflows with spn. 🚀

---

## What You Learned

- ✅ Installed spn via Homebrew/Cargo/Docker
- ✅ Ran setup wizard and configured first provider
- ✅ Added MCP servers for extended LLM capabilities
- ✅ Synced configuration to AI editors
- ✅ Verified everything with status dashboard
- ✅ Explored next steps (local models, skills, Nika)

**Next Tutorial:** [Working with Nika Workflows](./NIKA_WORKFLOWS.md) (coming soon)

---

<div align="center">

**Made with 💜 and 🦀 by the SuperNovae team**

[🌟 Star us on GitHub](https://github.com/supernovae-st/supernovae-cli) • [📖 Full Docs](../README.md) • [🐙 Report Issues](https://github.com/supernovae-st/supernovae-cli/issues)

</div>
