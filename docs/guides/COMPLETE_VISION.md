# SuperNovae — Vision Complète pour Débutants

**Date:** 2026-03-02
**Pour:** Quelqu'un qui ne connaît rien au projet

---

## 🎯 L'Analogie Fondamentale

```
┌────────────────────────────────────────────────────────────────┐
│  DÉVELOPPEMENT WEB CLASSIQUE                                   │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  npm (Package Manager)  →  Tu installes des librairies        │
│  node (Runtime)         →  Tu exécutes du JavaScript          │
│                                                                │
│  $ npm install express                                         │
│  $ node server.js                                              │
│                                                                │
└────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────┐
│  DÉVELOPPEMENT IA AVEC SUPERNOVAE                              │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  spn (Package Manager)  →  Tu installes des workflows IA      │
│  nika (Runtime)         →  Tu exécutes des workflows IA       │
│  novanet (Knowledge)    →  Base de données intelligente       │
│                                                                │
│  $ spn add @workflows/seo-audit                                │
│  $ nika run @workflows/seo-audit                               │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

**En une phrase:** SuperNovae c'est "npm + node pour l'IA", avec une base de connaissances intelligente (novanet) en bonus.

---

## 🧩 Les 3 Pièces du Puzzle

### 1. **spn** — Le Package Manager (l'App Store)

**C'est quoi?** Un gestionnaire de packages pour workflows IA, agents, prompts, etc.

**Ça fait quoi?**
- Tu **cherches** des packages: `spn search seo`
- Tu **installes** des packages: `spn add @workflows/seo-audit`
- Tu **gères** des dépendances: comme npm avec `package.json` → `spn.yaml`

**Les packages c'est quoi?**
| Type | Exemple | C'est quoi? |
|------|---------|-------------|
| `@workflows/` | `seo-audit` | Un workflow complet (plusieurs étapes IA) |
| `@agents/` | `researcher` | Un agent conversationnel multi-tours |
| `@prompts/` | `seo-meta` | Un template de prompt réutilisable |
| `@jobs/` | `daily-report` | Une tâche automatisée en arrière-plan |
| `@skills/` | `brainstorming` | Une compétence pour Claude Code |
| `@mcp/` | `neo4j` | Un serveur MCP (Model Context Protocol) |

**Où ça vit?**
```
~/.spn/packages/              ← Installation GLOBALE (une fois pour tous tes projets)
  └── @workflows/
      └── seo-audit/
          └── 1.2.0/
              ├── workflow.nika.yaml    ← Le workflow
              ├── README.md
              └── spn.json              ← Métadonnées

project/
├── spn.yaml                  ← Liste des packages que TU utilises (comme package.json)
├── spn.lock                  ← Versions exactes (comme package-lock.json)
```

---

### 2. **nika** — Le Runtime (le Moteur)

**C'est quoi?** Un moteur d'exécution pour workflows IA. Il prend un fichier YAML et l'exécute.

**Ça fait quoi?**
- Tu **exécutes** des workflows: `nika run seo-audit`
- Tu **crées** des workflows: `nika init`
- Tu **testes** des workflows: `nika check workflow.nika.yaml`

**Un workflow c'est quoi?** Un fichier YAML qui décrit une séquence d'actions IA.

**Exemple simple:**
```yaml
# hello.nika.yaml
schema: nika/workflow@0.9

tasks:
  - id: greet
    infer: "Say hello to Thibaut in French"

  - id: translate
    infer: "Translate '{{greet}}' to English"
```

**Exécution:**
```bash
$ nika run hello.nika.yaml

🚀 Running workflow: hello
✓ Task 1/2: greet
  → "Bonjour Thibaut!"

✓ Task 2/2: translate
  → "Hello Thibaut!"

✅ Workflow completed in 2.3s
```

**Les 5 verbes magiques de Nika:**

| Verbe | Ça fait quoi? | Exemple |
|-------|---------------|---------|
| `infer:` | Demande à un LLM (Claude, GPT, etc.) | `infer: "Résume ce texte"` |
| `exec:` | Exécute du code shell | `exec: curl https://example.com` |
| `fetch:` | Récupère des données web | `fetch: { url: "https://api.com" }` |
| `invoke:` | Appelle un outil externe | `invoke: novanet_generate` |
| `agent:` | Lance un agent conversationnel | `agent: { pkg: @agents/researcher }` |

**Où ça vit?**
```
project/
├── .nika/                    ← Configuration Nika
│   ├── config.toml           ← Quel LLM utiliser? (Claude, OpenAI, etc.)
│   ├── workflows/            ← Tes workflows LOCAUX (écrits à la main)
│   ├── agents/               ← Tes agents LOCAUX
│   └── context/              ← Fichiers de contexte
│
└── workflows/                ← Workflows à la racine (optionnel)
    └── main.nika.yaml
```

---

### 3. **novanet** — La Base de Connaissances (le Cerveau)

**C'est quoi?** Un knowledge graph (graphe de connaissances) avec Neo4j qui stocke des informations intelligentes.

**Ça fait quoi?**
- **Stocke** des entités sémantiques (ex: "QR Code", "SEO", "Restaurant")
- **Génère** du contenu natif en 200+ langues (français, anglais, japonais, etc.)
- **Expose** des outils via MCP que Nika peut utiliser

**Exemple d'utilisation:**
```yaml
# Un workflow Nika qui utilise NovaNet
tasks:
  - id: get_context
    invoke: novanet_generate
    params:
      entity: "qr-code"
      locale: "fr-FR"
      forms: ["text", "title"]

  - id: create_page
    infer: "Create landing page with: {{get_context}}"
```

**Concepts clés:**
- **Entity** → Un concept (ex: "QR Code")
- **EntityNative** → Ce concept en français/anglais/etc.
- **Page** → Une structure de page (ex: "landing-page")
- **PageNative** → Cette page générée en français/anglais/etc.

**Où ça vit?**
```
novanet/
├── tools/novanet/            ← CLI Rust
└── mcp-server/               ← Serveur MCP (expose les outils)

brain/
├── models/                   ← Schéma YAML (61 NodeClasses, 182 ArcClasses)
└── seed/                     ← Données seed (Cypher queries)

Neo4j Database                ← Base de données graphe
```

---

## 🔗 Comment les 3 Pièces se Connectent

```
┌────────────────────────────────────────────────────────────────────┐
│  USER                                                              │
└────────────────────────────────────────────────────────────────────┘
     ↓
     ↓ spn search / spn add
     ↓
┌────────────────────────────────────────────────────────────────────┐
│  SPN (Package Manager)                                             │
├────────────────────────────────────────────────────────────────────┤
│  • Télécharge depuis registry (GitHub)                             │
│  • Installe dans ~/.spn/packages/                                  │
│  • Met à jour spn.yaml + spn.lock                                  │
└────────────────────────────────────────────────────────────────────┘
     ↓
     ↓ Packages installés
     ↓
┌────────────────────────────────────────────────────────────────────┐
│  NIKA (Runtime)                                                    │
├────────────────────────────────────────────────────────────────────┤
│  • Résout @workflows/name → ~/.spn/packages/...                    │
│  • Charge workflow.nika.yaml                                       │
│  • Exécute les tasks (infer, exec, fetch, invoke, agent)          │
│  • Peut appeler NovaNet via MCP                                    │
└────────────────────────────────────────────────────────────────────┘
     ↓
     ↓ invoke: novanet_generate
     ↓
┌────────────────────────────────────────────────────────────────────┐
│  NOVANET (Knowledge Graph)                                         │
├────────────────────────────────────────────────────────────────────┤
│  • Reçoit requête via MCP                                          │
│  • Query Neo4j pour entités/pages                                  │
│  • Génère contenu natif (fr-FR, en-US, etc.)                       │
│  • Retourne résultat à Nika                                        │
└────────────────────────────────────────────────────────────────────┘
     ↓
     ↓ Résultat
     ↓
┌────────────────────────────────────────────────────────────────────┐
│  LLM PROVIDER (Claude, OpenAI, etc.)                               │
├────────────────────────────────────────────────────────────────────┤
│  • Nika envoie prompt + contexte NovaNet                           │
│  • LLM génère réponse                                              │
│  • Nika récupère résultat                                          │
└────────────────────────────────────────────────────────────────────┘
     ↓
     ↓ Résultat final
     ↓
┌────────────────────────────────────────────────────────────────────┐
│  USER (reçoit le résultat)                                         │
└────────────────────────────────────────────────────────────────────┘
```

**Agnostique = Liberté totale:**
- **LLM Provider:** Claude, OpenAI, Mistral, Groq, DeepSeek, Ollama (local), etc.
- **Platform:** Linux, macOS, Windows (Rust = cross-platform)
- **Pas de lock-in!** Change de provider avec `--provider openai` ou dans config.

---

## 👤 User Journey 1: Dev Solo Découvre un Workflow

**Contexte:** Pierre est dev, il veut analyser le SEO de son site.

### Étape 0: Installation (une fois)
```bash
# Installer spn
$ brew install supernovae-st/tap/spn
# Ou: cargo install supernovae-cli

# Installer nika
$ brew install supernovae-st/tap/nika
# Ou: cargo install nika
```

### Étape 1: Découvrir (10 secondes)
```bash
$ spn search seo

🔍 Found 3 packages

  @workflows/seo-audit         v1.2.0  ⭐ 45
  Analyse SEO complète: meta tags, performance, accessibilité
  Downloads: 1,245 · Last updated: 3 days ago

  @agents/seo-researcher       v2.0.1  ⭐ 32
  Agent conversationnel qui trouve opportunités SEO
  Downloads: 890 · Last updated: 1 week ago

  @prompts/seo-meta            v0.5.0  ⭐ 18
  Template pour générer meta descriptions SEO
  Downloads: 450 · Last updated: 2 weeks ago
```

**Pierre:** "Cool! Je vais prendre `seo-audit`."

### Étape 2: Installer (10 secondes)
```bash
$ spn add @workflows/seo-audit

📦 Resolving @workflows/seo-audit...
   → Found latest version: 1.2.0

⬇️  Downloading from registry...
   → https://github.com/supernovae-st/registry/.../seo-audit-1.2.0.tar.gz

✓ Downloaded to ~/.spn/packages/@workflows/seo-audit/1.2.0/
✓ Added to spn.yaml:
   dependencies:
     @workflows/seo-audit: "^1.2.0"
✓ Updated spn.lock
✓ Available for nika

📝 Run with: nika run @workflows/seo-audit
```

**Qu'est-ce qui s'est passé?**
1. **spn** a cherché le package dans le registry (GitHub)
2. Téléchargé le tarball (.tar.gz)
3. Extrait dans `~/.spn/packages/`
4. Ajouté la dépendance à `spn.yaml` (créé si n'existe pas)
5. Créé/mis à jour `spn.lock` avec la version exacte

**Fichiers créés:**
```
~/.spn/packages/
  └── @workflows/
      └── seo-audit/
          └── 1.2.0/
              ├── workflow.nika.yaml    ← Le workflow
              ├── README.md
              └── spn.json

project/
├── spn.yaml                  ← Nouveau!
└── spn.lock                  ← Nouveau!
```

### Étape 3: Utiliser (10 secondes)
```bash
$ nika run @workflows/seo-audit --url https://qrcode-ai.com

🚀 Running workflow: SEO Audit (v1.2.0)

✓ Task 1/5: fetch_page
  → Fetched HTML (342 KB)

✓ Task 2/5: analyze_meta
  → Title: 8/10 (48 chars, optimal: 50-60)
  → Description: 9/10 (155 chars, optimal: 150-160)
  → Keywords: 7/10 (some missing)

✓ Task 3/5: check_performance
  → First Contentful Paint: 1.2s (Good)
  → Largest Contentful Paint: 2.1s (Good)
  → Cumulative Layout Shift: 0.05 (Good)
  → Total Score: 95/100

✓ Task 4/5: check_accessibility
  → Alt text: 98% (1 missing)
  → ARIA labels: 100%
  → Color contrast: 100%
  → Total Score: 98/100

✓ Task 5/5: generate_report
  → Report saved to: seo-audit-report.md

✅ Workflow completed in 8.3s

📊 SEO Score: 95/100 (Excellent!)
```

**Pierre:** "Wow! Ça marche direct, sans config!"

**Total time:** 30 secondes. 3 commandes. Ça marche.

---

## 👥 User Journey 2: Team Lead Standardise Workflows

**Contexte:** Marie dirige une équipe de 5 devs. Elle veut que tout le monde utilise les mêmes workflows.

### Étape 1: Créer le Manifest d'Équipe
```bash
$ cd team-project
$ cat > spn.yaml <<EOF
name: team-seo-tools
version: 1.0.0
description: "Workflows SEO standardisés pour l'équipe"

dependencies:
  @workflows/seo-audit: "^1.2.0"
  @workflows/content-generator: "^2.1.0"
  @agents/seo-researcher: "^3.0.0"
  @prompts/meta-description: "^0.8.0"

dev_dependencies:
  @workflows/test-runner: "^1.0.0"
EOF

$ git add spn.yaml
$ git commit -m "Add team workflow dependencies"
$ git push
```

### Étape 2: Les Membres de l'Équipe Installent
```bash
# Dev #1 (Alice)
$ git pull
$ spn install

📦 Installing 5 packages from spn.yaml...

⬇️  @workflows/seo-audit@1.2.0
⬇️  @workflows/content-generator@2.1.3
⬇️  @agents/seo-researcher@3.0.1
⬇️  @prompts/meta-description@0.8.2
⬇️  @workflows/test-runner@1.0.5

✓ Installed 5 packages
✓ Created spn.lock (commit this!)

→ All team members will use exact same versions (from spn.lock)
```

**Qu'est-ce qui s'est passé?**
1. `spn install` lit `spn.yaml`
2. Résout les versions selon contraintes (`^1.2.0` = "1.2.x compatible")
3. Télécharge tous les packages
4. Crée `spn.lock` avec versions **EXACTES**
5. Tout le monde qui fait `spn install` aura les **mêmes versions**

### Étape 3: Utilisation en Équipe
```bash
# Alice
$ nika run @workflows/seo-audit --url https://client-site.com

# Bob
$ nika run @workflows/content-generator --topic "SEO best practices"

# Charlie
$ nika run @agents/seo-researcher
→ (Lance agent conversationnel)

# Même versions pour tout le monde! ✅
```

### Étape 4: Mise à Jour Contrôlée
```bash
# Marie met à jour un package
$ spn update @workflows/seo-audit

📦 Updating @workflows/seo-audit...
   Current: 1.2.0
   Latest: 1.3.0

✓ Updated to 1.3.0
✓ Updated spn.lock

$ git add spn.lock
$ git commit -m "chore: update seo-audit to 1.3.0"
$ git push

# Team members
$ git pull
$ spn install   # Installe la nouvelle version
```

---

## 🏢 User Journey 3: Agency Crée des Workflows Customs

**Contexte:** Une agence veut créer un workflow custom qui combine packages + code local.

### Étape 1: Init Projet
```bash
$ mkdir client-seo-toolkit
$ cd client-seo-toolkit
$ nika init

? Project type:
  ❯ Empty - Minimal config only
    Content Generation - SEO, blog posts
    Code Automation - Review, refactor, tests
    Research Pipeline - Web scraping, analysis

→ [Sélectionne: Content Generation]

? Install starter workflows? (Y/n) y

? Select workflows to include: (Space to select)
  ❯ [x] @workflows/content-generator
    [x] @workflows/seo-audit
    [ ] @workflows/competitor-analysis

📦 Installing 2 workflows...

✓ Created .nika/
   ├── config.toml              ← Provider config (Claude par défaut)
   ├── policies.yaml            ← Security policies
   ├── workflows/               ← Tes workflows locaux
   ├── agents/                  ← Tes agents locaux
   └── context/                 ← Context files

✓ Installed @workflows/content-generator@2.1.3
✓ Installed @workflows/seo-audit@1.2.0
✓ Added to spn.yaml

→ Get started: nika run @workflows/content-generator
```

### Étape 2: Créer Workflow Custom Qui Utilise Packages
```bash
$ cat > .nika/workflows/client-seo-pipeline.nika.yaml <<'EOF'
schema: nika/workflow@0.9

# Import workflows depuis packages
include:
  - pkg: @workflows/seo-audit
    prefix: audit_

  - pkg: @workflows/content-generator
    prefix: content_

# Ton workflow custom
tasks:
  # 1. Audit SEO du site
  - id: run_audit
    invoke: audit_main
    params:
      url: "{{env.CLIENT_URL}}"

  # 2. Analyser résultats
  - id: analyze_results
    use: { audit: run_audit }
    infer: |
      Analyse les résultats SEO:
      {{use.audit}}

      Liste les 3 problèmes prioritaires.

  # 3. Générer contenu optimisé
  - id: generate_content
    use: { analysis: analyze_results }
    invoke: content_generate
    params:
      topic: "Solutions pour: {{use.analysis}}"
      tone: "professional"

  # 4. Créer rapport client
  - id: create_report
    use:
      audit: run_audit
      analysis: analyze_results
      content: generate_content
    exec: |
      cat > client-report.md <<REPORT
      # SEO Analysis Report

      ## Audit Results
      {{use.audit}}

      ## Priority Issues
      {{use.analysis}}

      ## Recommended Content
      {{use.content}}

      Generated on: $(date)
      REPORT

      echo "Report saved: client-report.md"
EOF
```

**Qu'est-ce qui se passe ici?**
1. **include:** Import des workflows depuis packages (comme `import` en JS)
2. **prefix:** Évite les conflits de noms (`audit_main`, `content_generate`)
3. **Tasks custom:** Ton propre workflow qui **compose** les packages
4. **use:** Passer résultats d'une task à l'autre

### Étape 3: Exécuter le Pipeline Custom
```bash
$ CLIENT_URL=https://client-site.com nika run client-seo-pipeline

🚀 Running workflow: client-seo-pipeline

✓ Task 1/4: run_audit (from @workflows/seo-audit)
  → SEO Score: 78/100
  → Issues found: 8

✓ Task 2/4: analyze_results
  → Priority 1: Missing meta descriptions (5 pages)
  → Priority 2: Slow mobile performance (LCP 3.2s)
  → Priority 3: Broken internal links (12)

✓ Task 3/4: generate_content (from @workflows/content-generator)
  → Generated 3 optimized meta descriptions
  → Generated performance tips
  → Generated link fix checklist

✓ Task 4/4: create_report
  → Report saved: client-report.md

✅ Workflow completed in 42.5s
```

**L'agence peut:**
- ✅ Réutiliser packages communautaires
- ✅ Créer workflows customs
- ✅ Combiner les deux (include)
- ✅ Versionner avec git
- ✅ Partager avec clients

---

## 🧠 User Journey 4: Utiliser NovaNet pour Contenu Natif

**Contexte:** Tu veux générer une landing page SEO en français ET anglais avec contexte intelligent.

### Étape 1: NovaNet Contient les Entités
```
Neo4j Database (NovaNet)
├── Entity: "qr-code"
│   ├── EntityNative (fr-FR): "Code QR"
│   ├── EntityNative (en-US): "QR Code"
│   ├── EntityNative (ja-JP): "QRコード"
│   └── Relations: [is-a] → "technology"
│                  [used-for] → "mobile-payment"
│                  [used-for] → "marketing"
└── Page: "landing-page"
    └── Structure: hero, features, benefits, cta
```

### Étape 2: Workflow Nika Utilise NovaNet
```yaml
# generate-landing.nika.yaml
schema: nika/workflow@0.9

tasks:
  # 1. Get context from NovaNet (français)
  - id: context_fr
    invoke: novanet_generate
    params:
      entity: "qr-code"
      locale: "fr-FR"
      forms: ["text", "title", "abbrev"]

  # 2. Generate landing page (français)
  - id: landing_fr
    use: { ctx: context_fr }
    infer: |
      Create a landing page for QR codes using this context:
      {{use.ctx}}

      Include: hero, 3 features, 2 benefits, CTA

  # 3. Get context from NovaNet (anglais)
  - id: context_en
    invoke: novanet_generate
    params:
      entity: "qr-code"
      locale: "en-US"
      forms: ["text", "title", "abbrev"]

  # 4. Generate landing page (anglais)
  - id: landing_en
    use: { ctx: context_en }
    infer: |
      Create a landing page for QR codes using this context:
      {{use.ctx}}

      Include: hero, 3 features, 2 benefits, CTA

  # 5. Save both versions
  - id: save
    use:
      fr: landing_fr
      en: landing_en
    exec: |
      echo "{{use.fr}}" > landing-fr.html
      echo "{{use.en}}" > landing-en.html
```

### Étape 3: Exécution
```bash
$ nika run generate-landing

🚀 Running workflow: generate-landing

✓ Task 1/5: context_fr
  → NovaNet query: Entity(qr-code) × Locale(fr-FR)
  → Retrieved: "Code QR, technologie de codage..."

✓ Task 2/5: landing_fr
  → Generated landing page (2,450 words)

✓ Task 3/5: context_en
  → NovaNet query: Entity(qr-code) × Locale(en-US)
  → Retrieved: "QR Code, encoding technology..."

✓ Task 4/5: landing_en
  → Generated landing page (2,380 words)

✓ Task 5/5: save
  → Saved: landing-fr.html
  → Saved: landing-en.html

✅ Workflow completed in 18.7s
```

**Pourquoi c'est puissant?**
- **NovaNet** stocke le contexte sémantique (qu'est-ce qu'un QR code?)
- **Nika** orchestre (récupère contexte → génère pages → sauvegarde)
- **LLM** génère (utilise contexte NovaNet pour être précis)
- **Résultat:** Contenu natif de qualité en 200+ langues

---

## 🔧 User Journey 5: Dev Avancé — Workflow Complexe

**Contexte:** Créer un workflow qui fait du web scraping, analyse, génère contenu, et publie.

```yaml
# research-publish-pipeline.nika.yaml
schema: nika/workflow@0.9

# Import packages
include:
  - pkg: @workflows/web-scraper
    prefix: scrape_
  - pkg: @agents/researcher
    prefix: research_
  - pkg: @workflows/content-generator
    prefix: content_

tasks:
  # 1. Scrape competitor sites
  - id: scrape_competitors
    invoke: scrape_main
    params:
      urls:
        - "https://competitor1.com"
        - "https://competitor2.com"
        - "https://competitor3.com"
      selectors:
        - ".pricing"
        - ".features"

  # 2. Launch research agent
  - id: research
    use: { data: scrape_competitors }
    agent:
      pkg: @agents/researcher
      prompt: |
        Analyze competitor data:
        {{use.data}}

        Find unique value propositions we can offer.
      max_turns: 10

  # 3. Get NovaNet context
  - id: novanet_context
    invoke: novanet_generate
    params:
      entity: "saas-pricing"
      locale: "en-US"

  # 4. Generate pricing strategy
  - id: pricing_strategy
    use:
      research: research
      context: novanet_context
    infer: |
      Based on:
      - Competitor analysis: {{use.research}}
      - Market context: {{use.context}}

      Propose a pricing strategy with 3 tiers.

  # 5. Generate marketing content
  - id: content
    use: { strategy: pricing_strategy }
    invoke: content_generate
    params:
      topic: "Pricing strategy: {{use.strategy}}"
      format: "landing-page"

  # 6. Publish (example: save to file)
  - id: publish
    use: { content: content }
    exec: |
      # Save to file
      echo "{{use.content}}" > docs/pricing.md

      # Commit to git
      git add docs/pricing.md
      git commit -m "feat: update pricing page"

      # Could also publish to CMS, send to API, etc.
      echo "Published to docs/pricing.md"
```

**Exécution:**
```bash
$ nika run research-publish-pipeline

🚀 Running workflow: research-publish-pipeline

✓ Task 1/6: scrape_competitors (3.2s)
  → Scraped 3 sites, 247 pricing entries

✓ Task 2/6: research (28.5s)
  → Agent completed 8 turns
  → Found 5 unique value propositions

✓ Task 3/6: novanet_context (0.8s)
  → Retrieved SaaS pricing context

✓ Task 4/6: pricing_strategy (5.3s)
  → Generated 3-tier pricing

✓ Task 5/6: content (12.1s)
  → Generated landing page (3,200 words)

✓ Task 6/6: publish (1.2s)
  → Committed to git: docs/pricing.md

✅ Workflow completed in 51.1s
```

**Ce workflow:**
- ✅ Scrape le web
- ✅ Lance un agent IA multi-tours
- ✅ Utilise NovaNet pour contexte
- ✅ Génère contenu
- ✅ Publie automatiquement

**Tout ça en 50 lignes de YAML!**

---

## 📋 Le Plan d'Implémentation (Version Simple)

### État Actuel (Mars 2026)

**Ce qui marche:**
- ✅ `spn add @workflows/name` télécharge et installe
- ✅ `spn skill add brainstorming` (proxy skills.sh)
- ✅ `spn mcp add neo4j` (proxy npm)
- ✅ `nika run local-workflow.nika.yaml` exécute workflows locaux
- ✅ `nika init` crée structure .nika/

**Ce qui manque:**
- ❌ `nika run @workflows/name` → ne résout pas les packages
- ❌ `include: { pkg: @workflows/name }` → pas supporté
- ❌ Bugs: `spn add` panic, `nika init` génère exemples invalides

### Plan v0.7.0 (4 Semaines)

#### **Semaine 1: Réparer les Bugs** 🔴

**Jours 1-2: Fix `spn add` tokio panic**
- Problème: Runtime async dropé incorrectement
- Solution: Refactor pour éviter blocking context
- Résultat: `spn add` marche sans crash

**Jours 2-3: Fix `nika init` invalid examples**
- Problème: Templates YAML invalides
- Solution: Mettre à jour templates vers syntaxe valide
- Résultat: `nika init` génère workflows qui marchent

**Jours 3-7: Créer le Résolveur**
- Créer module `resolver.rs`
- Fonction: `resolve_package_path("@workflows/name") → PathBuf`
- Résolution: `.nika/workflows/` → `~/.spn/packages/` → filesystem
- Tests: 23 unit tests

**Résultat Semaine 1:**
- ✅ Zero bugs
- ✅ Résolveur de base prêt

---

#### **Semaine 2: Résolution de Packages** 🟡

**Jours 8-9: Intégrer dans `nika run`**
```rust
// Avant
let path = file;  // Juste le chemin passé

// Après
let path = if file.starts_with('@') {
    resolve_package_path(file).await?  // Résout @workflows/name
} else {
    file
};
```

**Jours 10-11: Support @agents/**
- Résoudre `agent: { pkg: @agents/researcher }`
- Charger `agent.md` depuis package

**Jours 12-14: Support @prompts/ et @jobs/**
- Résoudre `@prompts/name` → `prompt.md`
- Résoudre `@jobs/name` → `job.nika.yaml`

**Résultat Semaine 2:**
- ✅ `nika run @workflows/name` marche!
- ✅ Tous les types de packages résolus

---

#### **Semaine 3: Includes et Optimisation** 🟢

**Jours 15-16: Support `pkg:` dans includes**
```yaml
# Avant (seulement path)
include:
  - path: ./local/workflow.nika.yaml

# Après (path OU pkg)
include:
  - path: ./local/workflow.nika.yaml
  - pkg: @workflows/seo-audit
    prefix: seo_
```

**Jours 17-21: Polish**
- Path traversal security (valider `..` dans noms)
- Checksum verification (SHA256)
- Race condition fixes (atomic renames)

**Résultat Semaine 3:**
- ✅ Workflow composition marche
- ✅ Sécurité renforcée

---

#### **Semaine 4: CLI Interactif et Release** 🚀

**Jours 22-24: Interactive `spn add`**
```bash
$ spn add

? What type of package? (Use arrow keys)
  ❯ Workflow - Complete Nika workflows
    Agent - Multi-turn agents
    Prompt - Prompt templates
    Job - Background tasks

? Search query: seo

🔍 Searching...
? Select package:
  ❯ @workflows/seo-audit v1.2.0  ⭐ 45

📦 Installing...
✓ Installed

? Run now? (Y/n)
```

**Jours 24-25: Interactive `nika init`**
```bash
$ nika init

? Project type:
  ❯ Content Generation
    Code Automation
    Research Pipeline

? Install starter workflows?
  [x] @workflows/content-generator
  [ ] @workflows/code-review

✓ Created .nika/
✓ Installed 1 package
```

**Jours 26-28: Release**
- Tests: 35+ integration tests
- Docs: Guides utilisateur
- Release: v0.7.0 tag

**Résultat Semaine 4:**
- ✅ UX guidée pour débutants
- ✅ v0.7.0 released!

---

## 🎯 Résumé Final: À Quoi Ça Ressemble

### Avant (Aujourd'hui)
```bash
# Tu dois tout faire à la main
$ curl https://raw.../workflow.yaml > workflow.nika.yaml
$ nika run workflow.nika.yaml
```

### Après v0.7.0 (Dans 4 Semaines)
```bash
# Découvre → Installe → Utilise (30 secondes)
$ spn search seo
$ spn add @workflows/seo-audit
$ nika run @workflows/seo-audit
```

---

## 🌟 Les 3 Forces de SuperNovae

### 1. **Agnostique = Liberté**
- **LLM:** Claude, OpenAI, Mistral, Groq, DeepSeek, Ollama
- **Platform:** Linux, macOS, Windows
- **Change en 1 ligne:** `--provider openai`

### 2. **Composable = Puissant**
- Combine packages communautaires
- Écris workflows customs
- Mix local + packages
- Inclut des workflows dans workflows

### 3. **Simple = Rapide**
- 3 commandes pour être productif
- 30 secondes de débutant à résultat
- YAML lisible par humains
- Pas de code, juste de la logique

---

## 🚀 Vision Finale

```
SuperNovae = npm pour l'IA
├── spn     → Package manager (cherche, installe, gère)
├── nika    → Runtime (exécute workflows IA)
└── novanet → Knowledge graph (contexte intelligent)

🎯 Mission: Rendre l'IA accessible et réutilisable
   - Solo devs: Produis vite
   - Teams: Standardise workflows
   - Agencies: Crée custom pipelines
   - Entreprises: Scale IA
```

**En 4 semaines, on passe de "système fragmenté" à "écosystème complet".**

**Prêt à construire ça? 🦸**

---

## 📚 Annexe: Référence Complète des Commandes

### spn — SuperNovae Package Manager (54 commandes)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  spn — SuperNovae Package Manager (54 commandes)                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PACKAGE MANAGEMENT (Core)                                                  │
│  ├── spn add <package>              Ajouter un package (@workflows/name)   │
│  ├── spn remove <package>           Supprimer un package                   │
│  ├── spn install [--frozen]         Installer depuis spn.yaml              │
│  ├── spn update [package]           Mettre à jour packages                 │
│  ├── spn outdated                   Lister packages obsolètes              │
│  ├── spn search <query>             Chercher dans le registry              │
│  ├── spn info <package>             Infos sur un package                   │
│  ├── spn list                       Lister packages installés              │
│  ├── spn publish [--dry-run]        Publier un package                     │
│  └── spn version <bump>             Bumper version (major/minor/patch)     │
│                                                                             │
│  SKILLS (Proxy skills.sh)                                                   │
│  ├── spn skill add <name>           Télécharger skill depuis skills.sh     │
│  ├── spn skill remove <name>        Supprimer un skill                     │
│  ├── spn skill list                 Lister skills installés                │
│  └── spn skill search <query>       Chercher skills sur skills.sh          │
│                                                                             │
│  MCP SERVERS (Proxy npm)                                                    │
│  ├── spn mcp add <name> [-g/-p]     Installer MCP server (npm)             │
│  ├── spn mcp remove <name>          Supprimer MCP server                   │
│  ├── spn mcp list [--json]          Lister MCP servers                     │
│  └── spn mcp test <name>            Tester connexion MCP                   │
│                                                                             │
│  NIKA PROXY (spn nk → nika)                                                │
│  ├── spn nk run <file>              → nika run <file>                      │
│  ├── spn nk check <file>            → nika check <file>                    │
│  ├── spn nk studio                  → nika studio                          │
│  ├── spn nk jobs start              → nika jobs start                      │
│  ├── spn nk jobs status             → nika jobs status                     │
│  └── spn nk jobs stop               → nika jobs stop                       │
│                                                                             │
│  NOVANET PROXY (spn nv → novanet)                                          │
│  ├── spn nv tui                     → novanet tui                          │
│  ├── spn nv query <query>           → novanet query <query>                │
│  ├── spn nv mcp start/stop          → novanet mcp start/stop               │
│  ├── spn nv add-node <name>         → novanet node create                  │
│  ├── spn nv add-arc <name>          → novanet arc create                   │
│  ├── spn nv override <name>         → novanet node override                │
│  ├── spn nv db start                → novanet db start (Neo4j)             │
│  ├── spn nv db seed                 → novanet db seed                      │
│  ├── spn nv db reset                → novanet db reset                     │
│  └── spn nv db migrate              → novanet db migrate                   │
│                                                                             │
│  CONFIGURATION                                                              │
│  ├── spn config show [section]      Voir config résolue                    │
│  ├── spn config where               Voir chemins des fichiers config       │
│  ├── spn config list [--show-origin] Lister config avec origines          │
│  └── spn config edit [--local/--user/--mcp]  Éditer config                │
│                                                                             │
│  SCHEMA (NovaNet)                                                           │
│  ├── spn schema status              État du schema                         │
│  ├── spn schema validate            Valider schema                         │
│  ├── spn schema resolve             Voir schema fusionné                   │
│  ├── spn schema diff                Diff vs dernière résolution            │
│  ├── spn schema exclude <name>      Exclure un node                        │
│  └── spn schema include <name>      Ré-inclure un node                     │
│                                                                             │
│  PROVIDER (API Keys)                                                        │
│  ├── spn provider list [--show-source]  Lister clés API (masquées)        │
│  ├── spn provider set <name>        Stocker clé dans Keychain              │
│  ├── spn provider get <name>        Récupérer clé (masquée)                │
│  ├── spn provider delete <name>     Supprimer clé                          │
│  ├── spn provider migrate           Migrer env vars → Keychain             │
│  └── spn provider test <name>       Tester connexion provider              │
│                                                                             │
│  SYNC (vers éditeurs - HORS SCOPE v0.7)                                    │
│  ├── spn sync [--enable/--disable]  Activer/désactiver sync                │
│  ├── spn sync --status              Voir état sync                         │
│  ├── spn sync --target <editor>     Sync vers éditeur spécifique           │
│  └── spn sync --dry-run             Preview sans modifier                  │
│                                                                             │
│  UTILITY                                                                    │
│  ├── spn init [--local/--mcp]       Initialiser projet                     │
│  ├── spn doctor                     Diagnostic système                     │
│  ├── spn status [--json]            État écosystème                        │
│  └── spn topic [name]               Aide détaillée sur un sujet            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 🔗 Comment .nika Intègre spn (Détails Techniques)

### Vue d'Ensemble: AVANT vs APRÈS

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  AVANT v0.7.0 (Aujourd'hui)                                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  spn add @workflows/seo ─────────► ~/.spn/packages/@workflows/seo/1.0/     │
│                                           │                                 │
│                                           ✖ Pas de lien                    │
│                                           │                                 │
│  nika run @workflows/seo ─────────► ERREUR: "File not found"               │
│                                                                             │
│  .nika/ ne sait pas que ~/.spn/packages/ existe!                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  APRÈS v0.7.0 (Ce qu'on va faire)                                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  spn add @workflows/seo ─────────► ~/.spn/packages/@workflows/seo/1.0/     │
│                                           │                                 │
│                                           ✓ Résolveur!                     │
│                                           │                                 │
│  nika run @workflows/seo ─────────► Résout vers ~/.spn/packages/           │
│                                     └──► Exécute workflow.nika.yaml        │
│                                                                             │
│  Nika sait chercher dans ~/.spn/packages/!                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Structure des Dossiers Détaillée

```
~/.spn/                                 ← GLOBAL (géré par spn)
├── packages/                           ← Packages installés
│   ├── @workflows/
│   │   └── seo-audit/
│   │       └── 1.2.0/
│   │           ├── workflow.nika.yaml  ← LE WORKFLOW
│   │           ├── README.md
│   │           └── spn.json            ← Manifest
│   ├── @agents/
│   │   └── researcher/
│   │       └── 2.0.0/
│   │           ├── agent.md            ← L'AGENT
│   │           └── spn.json
│   ├── @prompts/
│   │   └── seo-meta/
│   │       └── 0.5.0/
│   │           ├── prompt.md           ← LE PROMPT
│   │           └── spn.json
│   └── @jobs/
│       └── daily-report/
│           └── 1.0.0/
│               ├── job.nika.yaml       ← LE JOB
│               └── spn.json
├── cache/
│   └── tarballs/                       ← Tarballs téléchargés
├── state.json                          ← État global
└── mcp.yaml                            ← Config MCP globale

project/                                ← PROJET (géré par user + spn)
├── spn.yaml                            ← Déclaration des dépendances
│   # dependencies:
│   #   @workflows/seo-audit: "^1.2.0"
│   #   @agents/researcher: "^2.0.0"
│
├── spn.lock                            ← Versions exactes (lockfile)
│   # packages:
│   #   - name: @workflows/seo-audit
│   #     version: 1.2.0
│   #     checksum: sha256:abc123
│
├── .nika/                              ← Config NIKA (géré par nika)
│   ├── config.toml                     ← Provider, permissions, etc.
│   ├── policies.yaml                   ← Politiques de sécurité
│   ├── user.yaml                       ← Profil utilisateur
│   ├── memory.yaml                     ← Config mémoire
│   │
│   ├── workflows/                      ← Workflows LOCAUX (écrits à la main)
│   │   └── custom.nika.yaml            ← Ton workflow perso
│   │
│   ├── agents/                         ← Agents LOCAUX
│   │   └── my-agent.md
│   │
│   ├── skills/                         ← Skills LOCAUX
│   │   └── my-skill.md
│   │
│   └── context/                        ← Fichiers de contexte
│       └── project.md
│
└── workflows/                          ← Workflows à la racine (optionnel)
    └── main.nika.yaml
```

### Ordre de Résolution (Direct Lookup)

```
nika run <reference>

┌─────────────────────────────────────────────────────────────────────────────┐
│  ÉTAPE 1: Parser la référence                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  "seo-audit"           → Nom simple (cherche local puis package)           │
│  "@workflows/seo"      → Référence package (@scope/name)                   │
│  "@workflows/seo@1.2"  → Référence package avec version                    │
│  "./local.nika.yaml"   → Chemin relatif (fichier direct)                   │
│  "/abs/path.yaml"      → Chemin absolu (fichier direct)                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  ÉTAPE 2: Rechercher (dans l'ordre)                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1️⃣  CACHE (DashMap)                                                        │
│      Cache mémoire: @workflows/seo → PathBuf                               │
│      Si trouvé ET pas expiré: utiliser ✓                                   │
│                                                                             │
│  2️⃣  LOCAL (.nika/workflows/)                                               │
│      Chercher: .nika/workflows/seo-audit.nika.yaml                         │
│      Si trouvé: utiliser ✓                                                 │
│                                                                             │
│  3️⃣  GLOBAL (~/.spn/packages/)                                              │
│      a) Lire spn.lock pour version exacte                                  │
│      b) Sinon: prendre dernière version installée                          │
│      c) Chercher: ~/.spn/packages/@workflows/seo-audit/1.2.0/              │
│      Si trouvé: utiliser ✓                                                 │
│                                                                             │
│  4️⃣  FILESYSTEM (chemin direct)                                             │
│      Si c'est un chemin: lire directement                                  │
│                                                                             │
│  5️⃣  ERREUR                                                                 │
│      "Package @workflows/seo not found. Run: spn add @workflows/seo"       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  ÉTAPE 3: Charger et exécuter                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Path résolu: ~/.spn/packages/@workflows/seo-audit/1.2.0/workflow.nika.yaml│
│                                                                             │
│  1. Lire le fichier YAML                                                   │
│  2. Parser le workflow                                                     │
│  3. Résoudre les includes (récursif)                                       │
│  4. Exécuter les tasks                                                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Tableau Récapitulatif: Qui Fait Quoi

```
┌──────────────────────┬────────────────────────────┬──────┬──────────────────────────────┐
│        Action        │          Commande          │ Qui  │              Où              │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Chercher packages    │ spn search seo             │ spn  │ Registry GitHub              │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Installer package    │ spn add @workflows/seo     │ spn  │ ~/.spn/packages/             │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Déclarer dépendance  │ (auto avec spn add)        │ spn  │ spn.yaml                     │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Verrouiller versions │ (auto avec spn install)    │ spn  │ spn.lock                     │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Exécuter workflow    │ nika run @workflows/seo    │ nika │ Résout vers ~/.spn/packages/ │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Exécuter local       │ nika run custom            │ nika │ .nika/workflows/             │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Valider workflow     │ nika check file.yaml       │ nika │ N/A                          │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Créer projet         │ nika init                  │ nika │ .nika/                       │
├──────────────────────┼────────────────────────────┼──────┼──────────────────────────────┤
│ Gérer API keys       │ spn provider set anthropic │ spn  │ OS Keychain                  │
└──────────────────────┴────────────────────────────┴──────┴──────────────────────────────┘
```

### Ce que v0.7.0 Va Ajouter

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  NOUVEAU dans v0.7.0                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  DANS NIKA (code à écrire):                                                │
│  ├── resolver.rs          Résoudre @workflows/name → chemin fichier        │
│  ├── cache.rs             Cache DashMap pour lookups rapides               │
│  ├── lockfile.rs          Lire spn.lock pour versions exactes              │
│  └── include.rs           Support pkg: dans includes                       │
│                                                                             │
│  DANS SPN (bug fixes):                                                      │
│  ├── add.rs               Fix tokio panic                                  │
│  └── (pas d'autre code)   spn marche déjà ✓                                │
│                                                                             │
│  L'INTÉGRATION = code dans NIKA pour lire ce que SPN installe              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Schéma Final: Le Flow Complet

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  FLOW COMPLET: spn add → nika run                                          │
└─────────────────────────────────────────────────────────────────────────────┘

   USER                    SPN                      NIKA                  LLM
    │                       │                        │                     │
    │  spn add @workflows/seo                        │                     │
    │──────────────────────►│                        │                     │
    │                       │                        │                     │
    │                       │ 1. Query registry      │                     │
    │                       │ 2. Download tarball    │                     │
    │                       │ 3. Extract to ~/.spn/  │                     │
    │                       │ 4. Update spn.yaml     │                     │
    │                       │ 5. Update spn.lock     │                     │
    │                       │                        │                     │
    │  ✓ Installed          │                        │                     │
    │◄──────────────────────│                        │                     │
    │                       │                        │                     │
    │  nika run @workflows/seo                       │                     │
    │──────────────────────────────────────────────►│                     │
    │                       │                        │                     │
    │                       │                        │ 1. Parse reference  │
    │                       │                        │ 2. Check cache      │
    │                       │                        │ 3. Read spn.lock    │
    │                       │                        │ 4. Find in ~/.spn/  │
    │                       │                        │ 5. Load YAML        │
    │                       │                        │ 6. Execute tasks ──►│
    │                       │                        │                     │
    │                       │                        │◄── LLM response ────│
    │                       │                        │                     │
    │  ✓ Result             │                        │                     │
    │◄──────────────────────────────────────────────│                     │
    │                       │                        │                     │
```

---

**Cette documentation complète est maintenant intégrée au guide. Prochaine étape: implémenter v0.7.0! 🚀**
