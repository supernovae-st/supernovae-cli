# SuperNovae CLI ↔ Nika Integration Plan

**Date:** 2026-03-02
**Status:** 🚧 In Progress
**Version:** v0.7.0 Target

---

## 🎯 Vision

**SuperNovae CLI = App Store pour packages IA, agnostique et universel**

- **SPN = Package Manager POUR NIKA** (priorité #1)
- Centralise workflows, agents, skills, MCP servers, jobs, prompts, schemas
- Utilisable par solo dev, team lead, agency, product builder
- **Agnostique:**
  - **LLM Provider:** Fonctionne avec Claude, OpenAI, Mistral, Groq, etc.
  - **Platform:** Linux, macOS, Windows (cross-platform)
- **Focus:** Intégration spn ↔ nika (100% du scope actuel)

---

## 📦 Package Types (MVP v0.7.0)

| Scope | Type | Utilisé dans | Storage | Proxy? | Format Fichier |
|-------|------|--------------|---------|--------|----------------|
| `@workflows/` | Workflows Nika | Nika run/include | `~/.spn/packages/` | Non (own) | `workflow.nika.yaml` |
| `@agents/` | Agents multi-turn | Nika agent verb | `~/.spn/packages/` | Non (own) | `agent.md` (YAML frontmatter) |
| `@prompts/` | Prompt templates | Nika tasks | `~/.spn/packages/` | Non (own) | `prompt.md` |
| `@jobs/` | Background jobs | Nika jobs daemon | `~/.spn/packages/` | Non (own) | `job.nika.yaml` |
| `@skills/` | Skills | Nika skills: | `~/.claude/skills/` | **Oui (skills.sh)** ✅ | `skill.md` |
| `@mcp/` | MCP servers | Nika mcp: | npm global | **Oui (npm)** ✅ | `package.json` (npm) |

**MVP (v0.7.0):** @workflows, @agents, @prompts, @jobs, @skills (proxy), @mcp (proxy)
**v2 (Q3 2026):** @schemas (NovaNet integration)

---

## 🖥️ Commandes CLI (État Actuel)

### SPN (54 commandes)

**Top-level:**
- `add`, `remove`, `install`, `update`, `outdated`, `search`, `info`, `list`
- `publish`, `version`, `sync`, `doctor`, `status`, `init`, `topic`

**Subcommands:**
- `skill`: add, remove, list, search (proxy skills.sh) ✅
- `mcp`: add, remove, list, test (proxy npm) ✅
- `nk`: run, check, studio, jobs (proxy nika)
- `nv`: tui, query, mcp, add-node, db (proxy novanet)
- `config`: show, where, list, edit
- `schema`: status, validate, resolve, diff, exclude, include
- `provider`: list, set, get, delete, migrate, test

### Nika (39 commandes)

**Top-level:**
- `run`, `check`, `init`, `ui`, `chat`, `studio`, `completion`, `doctor`

**Subcommands:**
- `trace`: list, show, export, clean
- `provider`: list, set, get, delete, migrate, test
- `mcp`: list, test, tools
- `config`: list, get, set, edit, path, reset
- `jobs`: start, stop, status, list, trigger, pause, resume, history, reload

### NovaNet (47 commandes)

**Top-level:**
- `tui`, `blueprint`, `data`, `overlay`, `query`, `search`, `export`
- `completions`, `doctor`, `stats`, `diff`

**Subcommands:**
- `node`: create, edit, delete
- `arc`: create, delete
- `schema`: generate, validate, cypher-validate, stats
- `doc`: generate
- `filter`: build
- `locale`: list, import, generate
- `db`: seed, migrate, reset, verify
- `knowledge`: generate, list
- `entity`: seed, list, validate
- `views`: export, validate

**Total: 140 commandes** across 3 CLIs

---

## 🏗️ Architecture Storage

### Global Storage (Installation unique)

```
~/.spn/
├── packages/                       # Packages installés
│   ├── @workflows/
│   │   └── seo-audit/
│   │       └── 1.0.0/
│   │           ├── workflow.nika.yaml
│   │           ├── README.md
│   │           └── spn.json        # Package manifest
│   ├── @agents/
│   │   └── researcher/
│   │       └── 2.0.0/
│   │           ├── agent.md        # Agent definition (YAML frontmatter)
│   │           └── spn.json
│   └── @jobs/
│       └── batch-process/
│           └── 1.0.0/
│               ├── job.nika.yaml
│               └── spn.json
├── cache/
│   └── tarballs/                   # Downloaded tarballs (cached)
├── state.json                      # Global installation state
└── mcp.yaml                        # Global MCP servers config

~/.claude/
└── skills/                         # Skills (proxy skills.sh)
    ├── brainstorming.md
    └── code-review.md
```

### Project Storage (Références locales)

```
project/
├── spn.yaml                        # Manifest des dépendances
│   dependencies:
│     @workflows/seo-audit: "^1.0"
│     @agents/researcher: "^2.0"
│     @skills/brainstorming: "*"   # Proxy (pas versionné)
│
├── spn.lock                        # Versions lockées
│   packages:
│     - name: @workflows/seo-audit
│       version: 1.0.2
│       checksum: sha256:abc123
│
├── .nika/                          # Config Nika + cache
│   ├── config.toml                 # Config (déjà existe)
│   ├── policies.yaml               # Security (déjà existe)
│   ├── user.yaml                   # User profile (déjà existe)
│   ├── memory.yaml                 # Memory config (déjà existe)
│   │
│   ├── .cache/                     # NOUVEAU: Cache de résolution
│   │   ├── workflows/
│   │   │   └── seo-audit -> ~/.spn/packages/@workflows/seo-audit/1.0.2/
│   │   └── agents/
│   │       └── researcher -> ~/.spn/packages/@agents/researcher/2.0.0/
│   │
│   ├── agents/                     # Agents locaux (écrits à la main OK)
│   │   └── custom-agent.md
│   ├── skills/                     # Skills locaux (écrits à la main OK)
│   │   └── custom-skill.md
│   ├── workflows/                  # Workflows locaux (écrits à la main OK)
│   │   └── custom-workflow.nika.yaml
│   └── context/                    # Context files (déjà existe)
│       └── project.md
│
└── workflows/                      # Workflows projet (root level)
    └── main.nika.yaml              # Peut importer packages
```

**Principe:** Les users PEUVENT ajouter à la main des agents/skills/workflows dans `.nika/`. Packages et local coexistent.

---

## 🔄 User Journey (Cas d'Usage Concrets)

### Cas 1: Découvrir et Installer un Workflow

```bash
# 1. DÉCOUVRIR
$ spn search seo
🔍 Found 3 packages

  @workflows/seo-audit         v1.2.0  ⭐ 45
  Analyse SEO complète (meta, perf, accessibilité)
  Downloads: 1.2K

  @agents/seo-researcher       v2.0.1  ⭐ 32
  Agent qui trouve opportunities SEO
  Downloads: 890

  @workflows/content-generator v0.8.0  ⭐ 18
  Génère content SEO-optimized
  Downloads: 450

# 2. INSTALLER
$ spn add @workflows/seo-audit
📦 Installing @workflows/seo-audit@1.2.0
   ✓ Downloaded to ~/.spn/packages/
   ✓ Added to spn.yaml
   ✓ Updated spn.lock
   ✓ Available for nika

# 3. UTILISER (3 façons)

# A. Direct par nom de package (PRÉFÉRENCE)
$ nika run @workflows/seo-audit --url https://qrcode-ai.com
🚀 Running SEO Audit...
✓ Meta tags: 8/10
✓ Performance: 95/100
✓ Accessibility: 98/100

# B. Import dans un workflow custom
$ cat > audit-qrcode.nika.yaml <<EOF
schema: nika/workflow@0.9
include:
  - pkg: @workflows/seo-audit    # Import depuis package
    prefix: seo_

tasks:
  - id: run_audit
    invoke: seo_audit
    params: { url: "https://qrcode-ai.com" }

  - id: send_report
    use: { audit: run_audit }
    exec: |
      echo "SEO Report:" > report.md
      echo "{{use.audit}}" >> report.md
EOF

$ nika run audit-qrcode.nika.yaml

# C. Init projet avec template
$ mkdir my-seo-project && cd my-seo-project
$ nika init --template @workflows/seo-audit
✓ Created .nika/config.toml
✓ Added seo-audit.nika.yaml
→ Run with: nika run seo-audit
```

### Cas 2: Créer un Workflow Local (Sans Package)

```bash
$ cd my-project
$ nika init                        # Crée .nika/ structure

$ cat > .nika/workflows/custom.nika.yaml <<EOF
schema: nika/workflow@0.9
tasks:
  - id: hello
    infer: "Say hello"
EOF

$ nika run .nika/workflows/custom.nika.yaml
# Ou si dans .nika/workflows/:
$ nika run custom                  # Auto-resolve dans .nika/workflows/
```

**Principe:** Packages ET fichiers locaux coexistent. Nika cherche dans:
1. `.nika/workflows/` (local)
2. `~/.spn/packages/` (global)
3. Chemins absolus/relatifs (filesystem)

### Cas 3: Utiliser un Agent Package

```bash
$ spn add @agents/researcher
$ nika run workflow-with-agent.nika.yaml

# workflow-with-agent.nika.yaml
schema: nika/workflow@0.9
tasks:
  - id: research
    agent:
      pkg: @agents/researcher      # Référence package
      prompt: "Find AI papers on LLMs"
      max_turns: 10
```

### Cas 4: Skills (Proxy skills.sh)

```bash
# Skills sont DÉJÀ proxiés vers skills.sh (implémenté ✅)
$ spn skill add brainstorming
✓ Downloaded from skills.sh
✓ Saved to ~/.claude/skills/brainstorming.md

# Utilisable dans workflows Nika:
schema: nika/workflow@0.9
skills:
  brainstorm: pkg:brainstorming    # Auto-resolve depuis ~/.claude/skills/
```

---

## 🛠️ Implémentation Technique

### Phase 1: Package Resolution (Semaine 1-2)

**Objectif:** `nika run @workflows/name` fonctionne

#### 1.1 Ajout de `resolve_workflow_path()` dans Nika

**Fichier:** `nika/tools/nika/src/main.rs`

**Ligne 727** (dans `run_workflow()`):

```rust
async fn run_workflow(
    file: &str,
    provider_override: Option<String>,
    model_override: Option<String>,
) -> Result<(), NikaError> {
    // NOUVEAU: Résolution de packages
    let resolved_path = if file.starts_with('@') {
        resolve_package_path(file).await?
    } else if !file.ends_with(".nika.yaml") && !file.contains('/') {
        // Essayer .nika/workflows/
        let local = Path::new(".nika/workflows").join(format!("{}.nika.yaml", file));
        if local.exists() {
            local.display().to_string()
        } else {
            file.to_string()
        }
    } else {
        file.to_string()
    };

    // Read and parse
    let yaml = tokio::fs::read_to_string(&resolved_path).await?;
    // ... reste du code inchangé
}
```

#### 1.2 Fonction `resolve_package_path()`

**Nouveau fichier:** `nika/tools/nika/src/registry/resolver.rs`

```rust
use std::path::{Path, PathBuf};
use crate::registry;

pub async fn resolve_package_path(package_ref: &str) -> Result<String, NikaError> {
    // Parse @scope/name[@version]
    let (name, version) = parse_package_ref(package_ref)?;

    // Determine version to use
    let version = if let Some(v) = version {
        v.to_string()
    } else {
        // Read from spn.lock if exists, else use latest installed
        resolve_version_from_lock(&name)
            .or_else(|| registry::installed_version(&name).ok().flatten())
            .ok_or_else(|| NikaError::PackageNotFound(name.clone()))?
    };

    // Build path: ~/.spn/packages/@scope/name/version/workflow.nika.yaml
    let pkg_dir = registry::package_dir(&name, &version)?;
    let workflow_file = pkg_dir.join("workflow.nika.yaml");

    if !workflow_file.exists() {
        return Err(NikaError::WorkflowNotFound(format!(
            "No workflow.nika.yaml in {}@{}", name, version
        )));
    }

    Ok(workflow_file.display().to_string())
}

fn parse_package_ref(input: &str) -> Result<(String, Option<String>), NikaError> {
    // @workflows/name → ("@workflows/name", None)
    // @workflows/name@1.0.0 → ("@workflows/name", Some("1.0.0"))

    if let Some((name, version)) = input.rsplit_once('@') {
        if name.starts_with('@') {
            Ok((name.to_string(), Some(version.to_string())))
        } else {
            Ok((format!("@{}", name), Some(version.to_string())))
        }
    } else {
        Ok((input.to_string(), None))
    }
}

fn resolve_version_from_lock(name: &str) -> Option<String> {
    // Read spn.lock from current dir
    let lock_path = Path::new("spn.lock");
    if !lock_path.exists() {
        return None;
    }

    let lock_content = std::fs::read_to_string(lock_path).ok()?;
    let lockfile: SpnLockfile = serde_yaml::from_str(&lock_content).ok()?;

    lockfile.packages
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.version.clone())
}
```

**Estimation:** ~100 lignes de code

#### 1.3 Support pour Agents/Jobs

Même logique pour `@agents/` et `@jobs/`:

- `@agents/name` → `~/.spn/packages/@agents/name/version/agent.md`
- `@jobs/name` → `~/.spn/packages/@jobs/name/version/job.nika.yaml`

Agent resolution dans `agent:` verb:

```yaml
tasks:
  - id: research
    agent:
      pkg: @agents/researcher   # Résoudre via registry
      prompt: "Find papers"
```

**Estimation:** ~50 lignes supplémentaires

---

### Phase 2: Include Package Support (Semaine 2-3)

**Objectif:** Inclure workflows depuis packages

#### 2.1 Modifier `IncludeSpec` pour supporter `pkg:`

**Fichier:** `nika/tools/nika/src/ast/include.rs`

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum IncludeSpec {
    Path {
        /// Relative path to workflow file
        path: String,

        /// Optional prefix for task IDs
        #[serde(default)]
        prefix: Option<String>,
    },
    Package {
        /// Package reference (e.g., @workflows/seo)
        pkg: String,

        /// Optional prefix for task IDs
        #[serde(default)]
        prefix: Option<String>,
    },
}
```

#### 2.2 Modifier `expand_includes_recursive()`

**Fichier:** `nika/tools/nika/src/ast/include_loader.rs`

```rust
fn expand_includes_recursive(...) -> Result<Workflow, NikaError> {
    for include_spec in includes {
        let include_path = match &include_spec {
            IncludeSpec::Path { path, .. } => {
                base_path.join(path)
            }
            IncludeSpec::Package { pkg, .. } => {
                // Resolve package to filesystem path
                let resolved = resolve_package_path(pkg).await?;
                PathBuf::from(resolved)
            }
        };

        // Rest of logic unchanged
    }
}
```

**Usage:**

```yaml
include:
  - path: ./local/tasks.nika.yaml   # Local file
    prefix: local_

  - pkg: @workflows/seo-tasks       # Package
    prefix: seo_
```

**Estimation:** ~30 lignes de modification

---

### Phase 3: spn Sync to .nika/ (Semaine 3)

**Objectif:** `spn install` crée symlinks dans `.nika/.cache/`

#### 3.1 Modifier `spn install` pour créer symlinks

**Fichier:** `supernovae-cli/src/commands/install.rs`

Après installation de chaque package (ligne ~150):

```rust
// After storage.install()
let installed = storage.install(&downloaded)?;

// NOUVEAU: Sync to .nika/ if exists
if Path::new(".nika").exists() {
    sync_package_to_nika(&installed)?;
}
```

#### 3.2 Fonction `sync_package_to_nika()`

**Nouveau fichier:** `supernovae-cli/src/sync/nika_sync.rs`

```rust
pub fn sync_package_to_nika(pkg: &InstalledPackage) -> Result<()> {
    let nika_cache = Path::new(".nika/.cache");

    // Determine package type
    let pkg_type = PackageType::from_scope(&pkg.name)
        .ok_or_else(|| SpnError::InvalidPackage(pkg.name.clone()))?;

    let cache_subdir = match pkg_type {
        PackageType::Workflow => "workflows",
        PackageType::Agent => "agents",
        PackageType::Job => "jobs",
        _ => return Ok(()), // Skip non-Nika packages
    };

    let link_dir = nika_cache.join(cache_subdir);
    std::fs::create_dir_all(&link_dir)?;

    // Extract short name (@workflows/seo-audit → seo-audit)
    let short_name = pkg.name.split('/').last().unwrap_or(&pkg.name);
    let link_path = link_dir.join(short_name);

    // Remove old symlink if exists
    if link_path.exists() || link_path.is_symlink() {
        std::fs::remove_file(&link_path).or_else(|_| std::fs::remove_dir_all(&link_path))?;
    }

    // Create symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&pkg.path, &link_path)?;

    println!("   ✓ Linked to .nika/.cache/{}/{}", cache_subdir, short_name);

    Ok(())
}
```

**Résultat:**

```bash
$ spn add @workflows/seo-audit
✓ Installed @workflows/seo-audit@1.2.0
✓ Linked to .nika/.cache/workflows/seo-audit -> ~/.spn/packages/...
```

**Estimation:** ~60 lignes de code

---

### Phase 4: CLI Interactif (Semaine 4)

**Objectif:** Expérience guidée pour découverte

#### 4.1 `spn add` interactif (sans args)

```bash
$ spn add
? What type of package? (Use arrow keys)
  ❯ Workflow - Complete Nika workflows
    Agent - Multi-turn agents
    Skill - Claude Code skills (proxy skills.sh)
    Job - Background tasks
    MCP Server - Model Context Protocol servers

? Search query: seo

🔍 Searching for "seo"...

? Select package to install: (Use arrow keys)
  ❯ @workflows/seo-audit v1.2.0  ⭐ 45
    Analyse SEO complète (meta, perf, accessibilité)

    @agents/seo-researcher v2.0.1  ⭐ 32
    Agent qui trouve opportunities SEO

📦 Installing @workflows/seo-audit@1.2.0...
✓ Installed successfully

? Run now? (Y/n) y

$ nika run @workflows/seo-audit
```

**Crate:** `dialoguer` (déjà populaire pour CLIs interactifs Rust)

**Estimation:** ~150 lignes de code

#### 4.2 `nika init` interactif

```bash
$ nika init
? Project type:
  ❯ Empty - Minimal config only
    Content Generation - SEO, blog posts
    Code Automation - Review, refactor, tests
    Research Pipeline - Web scraping, analysis
    Custom - I'll set it up myself

? Install starter workflows? (Y/n) y

? Select workflows to include: (Space to select)
  ❯ [x] @workflows/content-generator
    [ ] @workflows/code-review
    [x] @workflows/seo-audit

📦 Installing 2 workflows...
✓ Created .nika/config.toml
✓ Installed @workflows/content-generator
✓ Installed @workflows/seo-audit
✓ Added to spn.yaml

→ Get started: nika run @workflows/content-generator
```

**Estimation:** ~100 lignes de code

---

## 🐛 Bugs à Fixer (Priorité)

### Bug 1: `spn add` Tokio Panic

**Erreur:**
```
thread 'main' panicked at tokio-1.49.0/src/runtime/blocking/shutdown.rs:51:21:
Cannot drop a runtime in a context where blocking is not allowed.
```

**Cause:** Async runtime dropé incorrectement dans contexte synchrone.

**Fichier:** `supernovae-cli/src/commands/add.rs`

**Fix:** Refactor pour éviter blocking context:

```rust
// AVANT (problématique)
pub async fn run(package: String) -> Result<()> {
    // ...
    let client = IndexClient::new();  // Crée tokio runtime
    // ... runtime dropé ici
}

// APRÈS (fix)
#[tokio::main]
async fn main() -> Result<()> {
    // Runtime géré par tokio::main
}

// Ou utiliser Runtime::new() explicite
pub async fn run(package: String) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Logic here
    })
}
```

**Estimation:** 1-2 heures (debug + test)

### Bug 2: `nika init` Invalid Examples

**Erreur:**
```
× [NIKA-005] Schema validation failed: 3 errors
  [/tasks/0/output] Additional properties not allowed ('use.summary' was unexpected)
```

**Cause:** Workflow examples générés par `nika init` utilisent syntaxe invalide.

**Fichier:** `nika/tools/nika/src/main.rs` (ligne ~1312-1900, hardcoded templates)

**Fix:** Mettre à jour templates pour correspondre au schéma `nika/workflow@0.9`:

```yaml
# AVANT (invalide)
tasks:
  - id: summarize
    infer: "Summarize"
    output:
      use.summary: summary  # ❌ Invalide

# APRÈS (valide)
tasks:
  - id: summarize
    infer: "Summarize"

  - id: use_summary
    use: { summary: summarize }  # ✅ Valide
```

**Estimation:** 2-3 heures (update 4 templates + test)

---

## 📋 Roadmap d'Implémentation

### v0.7.0 (MVP - 4 semaines)

| Semaine | Tâches | LOC Estimé | Priorité |
|---------|--------|------------|----------|
| **S1** | Fix bugs (tokio panic, invalid examples) | ~100 | 🔴 P0 |
| **S1-S2** | Package resolution dans nika (workflows, agents, prompts, jobs) | ~200 | 🔴 P0 |
| **S2-S3** | Include package support (`pkg:` in includes) | ~30 | 🟡 P1 |
| **S3** | spn sync to .nika/ (symlinks) | ~60 | 🟡 P1 |
| **S4** | CLI interactif (spn add, nika init) | ~250 | 🟢 P2 |
| **S4** | Tests integration + docs | ~100 | 🟢 P2 |

**Total:** ~690 LOC + tests

### v0.8.0 (Q2 2026)

- Package groups (`@group/ai-dev`)
- Fuzzy search in registry
- Auto-update mechanism
- Publishing workflow (`spn publish`)

### v0.9.0 (Q3 2026)

- `@schemas/` + NovaNet integration
- Version conflict resolution
- Offline mode (cached registry)

---

## ✅ Checklist de Validation

Avant de considérer v0.7.0 complète:

- [ ] `spn add @workflows/name` fonctionne sans panic
- [ ] `nika run @workflows/name` résout et exécute
- [ ] `nika run @agents/name` résout et exécute
- [ ] Inclure `pkg: @workflows/name` dans workflows fonctionne
- [ ] `spn install` crée symlinks dans `.nika/.cache/`
- [ ] Packages ET fichiers locaux coexistent
- [ ] `nika init` génère exemples valides
- [ ] `spn add` interactif guide l'utilisateur
- [ ] `nika init --template` fonctionne
- [ ] Tests integration passent (100%)
- [ ] Documentation utilisateur complète

---

## 🎯 Succès Métrique

**Objectif:** Valider que l'intégration fonctionne avec un cas d'usage réel.

**User Story:**

```bash
# Dev crée un nouveau projet
mkdir qrcode-seo && cd qrcode-seo

# Découvre et installe workflow
spn search seo
spn add @workflows/seo-audit

# Utilise directement
nika run @workflows/seo-audit --url https://qrcode-ai.com

# ✅ Ça marche en 3 commandes, 30 secondes
```

**Si ça marche, on a réussi.**

---

## 📝 Notes

- **Skills.sh proxy:** DÉJÀ implémenté ✅ (`spn skill add brainstorming`)
- **MCP proxy:** DÉJÀ implémenté ✅ (`spn mcp add neo4j`)
- **Package types detection:** DÉJÀ implémenté ✅ (dans `add.rs`)
- **Workflow includes:** DÉJÀ implémenté ✅ (path only, pas pkg URI)

**Ce qui manque:**
- Résolution de packages dans `nika run`
- Include package support
- Sync vers .nika/

**Ce plan se concentre sur combler ces gaps.**

---

**Prêt à implémenter?** 🚀
