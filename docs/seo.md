# ADI Search Appearance & SEO Strategy

Domain: `https://adi.the-ihor.com`
Last updated: 2026-02-16

---

## Table of Contents

0. [Positioning & Core Message](#0-positioning--core-message)
1. [Brand Identity in Search](#1-brand-identity-in-search)
2. [Search Appearance Templates](#2-search-appearance-templates)
3. [Keyword Strategy](#3-keyword-strategy)
4. [Structured Data (Schema.org)](#4-structured-data-schemaorg)
5. [SERP Feature Targeting](#5-serp-feature-targeting)
6. [Technical SEO](#6-technical-seo)
7. [Content Strategy](#7-content-strategy)
8. [Link Building & Authority](#8-link-building--authority)
9. [Current Implementation Audit](#9-current-implementation-audit)
10. [Implementation Roadmap](#10-implementation-roadmap)
11. [Open & Community-First Messaging](#11-open--community-first-messaging)

---

## 0. Positioning & Core Message

### The One Sentence

> **Think agents. Think ADI.**

This is the brand anchor. Every page, every meta description, every talk, every tweet
reinforces this single mental association: **agents = ADI**.

### Positioning Stack

| Layer | Statement | Where it appears |
|-------|-----------|-----------------|
| **Tagline** | Think agents. Think ADI. | Hero, OG images, email signatures, conference slides, Twitter bio |
| **Identity** | The agent company. | About page, press kit, investor materials, LinkedIn company description |
| **Description** | Agent development infrastructure -- build, deploy, orchestrate. | Meta descriptions, README, directory listings |
| **Proof** | 30+ Rust libraries. Plugin architecture. Containerized runtime. Free. | Below-fold content, feature grids, comparison tables |

### How the Tagline Works for SEO

The phrase "Think agents. Think ADI." is designed to:

1. **Own the generic keyword "agents"** -- every repetition trains the association
2. **Work as a search seed** -- users who internalize it will search "ADI agents" (branded + category)
3. **Be snippet-friendly** -- short enough to appear in Google sitelink descriptions
4. **Transfer across formats** -- works spoken (podcasts, talks), written (articles, tweets), visual (OG images)
5. **Create a call-and-response pattern** -- "What do you use for agents?" / "ADI."

### Positioning Rules

- **Never lead with the technology** ("Rust-based modular platform..."). Lead with the outcome ("Build agents").
- **Never lead with features**. Lead with the category ownership ("The agent company").
- **"Agents" is the word we own.** Use it in every title, every H1, every first paragraph. Not "AI agents" everywhere -- just "agents" is enough once the association is built.
- **ADI is not a framework.** Frameworks are interchangeable. ADI is **infrastructure** -- foundational, load-bearing, permanent.
- **Keep the identity claim short.** "The agent company" -- 3 words. Don't dilute it with qualifiers ("The leading next-generation agent company for developers" -- no).

### Voice & Tone in Search Copy

| Do | Don't |
|----|-------|
| "Build agents." | "Leverage our cutting-edge platform to build AI agents." |
| "The agent company." | "A comprehensive agent development solution." |
| "Agents run on ADI." | "ADI provides infrastructure for running agents." |
| "30+ libraries. One CLI." | "An extensive collection of over 30 libraries." |
| "Free for individuals." | "We offer a generous free tier for individual developers." |

**The tone is**: Direct. Confident. Technical. No filler words. No superlatives. The product speaks.

### Tagline Deployment Map

| Surface | Format | Example |
|---------|--------|---------|
| Homepage hero | Large text + subheading | **Think agents. Think ADI.** / Agent development infrastructure |
| OG image (all pages) | Overlaid on brand visual | "Think agents. Think ADI." + adi.the-ihor.com |
| Twitter/X bio | @adi_agents | The agent company. Build, deploy, orchestrate. https://adi.the-ihor.com |
| GitHub org description | Plain text | Think agents. Think ADI. -- Agent development infrastructure in Rust. |
| CLI `--version` output | ASCII | `adi 1.x.x -- The agent company` |
| Email signature | Below name | ADI -- The agent company |
| Conference slides | First + last slide | Think agents. Think ADI. |
| README badges | Shield.io | `![ADI](https://img.shields.io/badge/ADI-The_Agent_Company-blue)` |
| `llms.txt` | First line | `# ADI -- The Agent Company` |
| Podcast intros | Spoken | "This is ADI -- the agent company." |
| 404 page | Playful variant | "This page doesn't exist. But your agents can. Think ADI." |

---

## 1. Brand Identity in Search

### Brand Name
- **Primary**: ADI (all caps, always)
- **Full form**: ADI - Agent Development Infrastructure
- **Identity claim**: The agent company.
- **Mantra**: Think agents. Think ADI.
- **Never**: Adi, adi (lowercase in user-facing contexts), A.D.I.

### Brand SERP Goal (Searching "ADI agents")
```
ADI - The Agent Company | Build, Deploy & Orchestrate AI Agents
https://adi.the-ihor.com
Think agents. Think ADI. Modular Rust infrastructure for autonomous
AI agents. 30+ libraries, plugin architecture, containerized runtime.
Free for individuals.

Sitelinks:
  Documentation          Pricing             Cocoon Runtime
  Quickstart Guide       Product Comparison  Plugin Registry
```

### Title Tag Formula
```
[Primary Keyword] - [Value Prop] | ADI
```
- Max 60 characters (Google truncates at ~580px)
- Brand "ADI" always at the end (tail brand position)
- Front-load the primary keyword
- Where space allows, echo the positioning ("...for AI Agents | ADI")

### Meta Description Formula
```
[Positioning hook]. [Action verb] [what]. [Differentiator]. [CTA].
```
- Target 150-155 characters (Google truncates at ~920px)
- Open with a positioning phrase when natural ("The agent company.", "Think agents.")
- Include primary + secondary keyword naturally
- Always include a call-to-action

---

## 2. Search Appearance Templates

### Homepage

```
Title:  ADI - The Agent Company | Build, Deploy & Orchestrate AI Agents
URL:    https://adi.the-ihor.com
Desc:   Think agents. Think ADI. Modular Rust infrastructure for autonomous
        AI agents -- 30+ libraries, plugin architecture, containerized
        runtime. Free for individuals. Start in 2 minutes.
```

### Pricing Page

```
Title:  ADI Pricing - Free, Pro & Team Plans | The Agent Company
URL:    https://adi.the-ihor.com/en/pricing
Desc:   Build agents for free. Pro at $29/mo for unlimited cocoons and
        priority support. Team at $99/mo. No credit card required.
        The agent company -- start today.
```

### Documentation Landing

```
Title:  ADI Documentation - Guides, API Reference & Tutorials
URL:    https://adi.the-ihor.com/en/docs
Desc:   Everything you need to build, deploy, and orchestrate agents.
        Installation guides, architecture deep-dives, API reference,
        and step-by-step tutorials. Get started in 5 minutes.
```

### Product Pages

#### Cocoon

```
Title:  Cocoon - Secure Containerized Runtime for AI Agents | ADI
URL:    https://adi.the-ihor.com/en/products/cocoon
Desc:   Run AI agents in isolated Docker containers with real-time WebSocket
        communication. Secure sandboxing, live command streaming, and
        automatic resource management. Part of ADI infrastructure.
```

#### Indexer

```
Title:  Code Indexer - Semantic Code Understanding with Embeddings | ADI
URL:    https://adi.the-ihor.com/en/products/indexer
Desc:   Give AI agents deep code understanding. Semantic indexing for Rust,
        Python, TypeScript, Go, Java, and C++. Graph-based symbol resolution
        with embedding search. Open source.
```

#### Knowledgebase

```
Title:  Knowledgebase - Persistent Agent Memory with Graph DB | ADI
URL:    https://adi.the-ihor.com/en/products/knowledgebase
Desc:   Graph-based knowledge storage with semantic search for AI agents.
        Persistent memory across sessions. Embeddings-powered retrieval.
        Build agents that learn and remember.
```

#### tsp-gen

```
Title:  tsp-gen - Pure Rust TypeSpec Code Generator | ADI
URL:    https://adi.the-ihor.com/en/products/tsp-gen
Desc:   Generate Rust, Python, TypeScript, and OpenAPI from TypeSpec
        definitions. Pure Rust implementation - no Node.js required.
        10x faster than the official TypeSpec compiler.
```

#### Analytics

```
Title:  Analytics - Real-Time AI Agent Metrics & Dashboards | ADI
URL:    https://adi.the-ihor.com/en/products/analytics
Desc:   Monitor AI agent performance with real-time analytics. Track task
        success rates, latency percentiles, and resource usage.
        Built on TimescaleDB for time-series precision.
```

### Doc Pages (Pattern)

```
Title:  [Topic] - ADI Documentation
URL:    https://adi.the-ihor.com/en/docs/[slug]
Desc:   [Actionable description of what the reader will learn]. Includes
        code examples, configuration reference, and best practices.
```

**Examples:**

```
Title:  Installation Guide - ADI Documentation
Desc:   Install ADI CLI in one command on macOS, Linux, or Windows.
        Supports Homebrew, curl install script, and cargo install.
        Complete setup in under 2 minutes.

Title:  Plugin Architecture - ADI Documentation
Desc:   Learn how ADI's unified v3 plugin ABI works. Build custom plugins
        with Rust async traits, type-safe contexts, and zero FFI overhead.
        Includes starter template and examples.

Title:  Agent Loop - ADI Documentation
Desc:   Configure and run autonomous LLM agents with ADI's agent loop.
        Supports tool calling, multi-step reasoning, and automatic retries.
        Works with any OpenAI-compatible API.
```

### Article/Blog Pages

```
Title:  [Article Title] | ADI Blog
URL:    https://adi.the-ihor.com/en/articles/[slug]
Desc:   [First compelling sentence from article]. [Read time]. By [Author].
```

### Comparison Pages (to create)

```
Title:  ADI vs LangChain - Agent Infrastructure Comparison | ADI
URL:    https://adi.the-ihor.com/en/comparisons/adi-vs-langchain
Desc:   ADI vs LangChain for agent development. Side-by-side: performance,
        deployment, plugin architecture, pricing. Think agents, think
        infrastructure -- not just a framework. Updated [month] 2026.
```

---

## 3. Keyword Strategy

### Primary Keywords (High Intent)

| Keyword | Volume | Difficulty | Intent | Target Page |
|---------|--------|------------|--------|-------------|
| ai agent framework | High | High | Commercial | Homepage |
| ai agent infrastructure | Med | Med | Commercial | Homepage |
| agent development platform | Med | Med | Commercial | Homepage |
| deploy ai agents | Med | Med | Commercial | /products/cocoon |
| llm agent orchestration | Med | Med | Commercial | /docs/agents |
| ai agent runtime | Low | Low | Commercial | /products/cocoon |
| code indexer for ai | Low | Low | Informational | /products/indexer |
| typespec code generator | Low | Low | Commercial | /products/tsp-gen |

### Long-Tail Keywords (Lower Competition, Higher Conversion)

| Keyword | Intent | Target Page |
|---------|--------|-------------|
| how to build autonomous ai agents | Informational | /docs/quickstart |
| rust ai agent framework | Commercial | Homepage |
| containerized ai agent execution | Commercial | /products/cocoon |
| semantic code search for llm | Informational | /products/indexer |
| ai agent persistent memory | Informational | /products/knowledgebase |
| self-hosted ai agent platform | Commercial | /docs/installation |
| ai agent plugin architecture | Informational | /docs/plugins |
| llm proxy bring your own key | Commercial | /docs/services |
| mcp server for ai agents | Commercial | /docs/mcp |
| ai agent monitoring dashboard | Commercial | /products/analytics |
| typespec rust alternative | Commercial | /products/tsp-gen |
| multi-agent system rust | Commercial | Homepage |

### Keyword Clusters (Content Pillars)

**Pillar 1: AI Agent Development** (Primary)
- Core topic page: `/docs/agents`
- Supporting: quickstart, architecture, agent-loop docs
- Blog: tutorials, patterns, best practices

**Pillar 2: AI Agent Deployment & Operations**
- Core topic page: `/products/cocoon`
- Supporting: executor docs, hive docs, analytics
- Blog: scaling agents, monitoring, security

**Pillar 3: AI Code Understanding**
- Core topic page: `/products/indexer`
- Supporting: knowledgebase, embeddings, language analyzers
- Blog: RAG for code, semantic search techniques

**Pillar 4: Developer Tools & Infrastructure**
- Core topic page: `/docs/architecture`
- Supporting: plugin system, CLI reference, API docs
- Blog: comparisons, migration guides, integration tutorials

### Branded Keywords to Protect

- `adi agent` / `adi ai`
- `adi cocoon`
- `adi indexer`
- `adi tsp-gen`
- `adi knowledgebase`
- `adi.the-ihor.com`

---

## 4. Structured Data (Schema.org)

### Currently Implemented
- `Organization` - basic info
- `WebSite` - with SearchAction
- `SoftwareApplication` - per product
- `BreadcrumbList` - auto from URL
- `TechArticle` - for docs
- `Article` - for blog (with speakable)
- `FAQPage` - for FAQ sections
- `HowTo` - for tutorials

### Additional Schema to Implement

#### 4.1 SoftwareSourceCode (for GitHub repos)

```json
{
  "@context": "https://schema.org",
  "@type": "SoftwareSourceCode",
  "name": "ADI CLI",
  "codeRepository": "https://github.com/adi-family/adi-cli",
  "programmingLanguage": "Rust",
  "runtimePlatform": ["macOS", "Linux", "Windows"],
  "license": "https://spdx.org/licenses/BSL-1.1.html",
  "author": {
    "@type": "Organization",
    "name": "ADI",
    "url": "https://adi.the-ihor.com"
  }
}
```

#### 4.2 Product (for pricing/conversion pages)

```json
{
  "@context": "https://schema.org",
  "@type": "Product",
  "name": "ADI Pro",
  "description": "Professional AI agent development infrastructure",
  "brand": { "@type": "Brand", "name": "ADI" },
  "offers": {
    "@type": "Offer",
    "price": "29.00",
    "priceCurrency": "USD",
    "priceValidUntil": "2027-12-31",
    "availability": "https://schema.org/InStock",
    "url": "https://adi.the-ihor.com/en/pricing"
  },
  "aggregateRating": {
    "@type": "AggregateRating",
    "ratingValue": "4.8",
    "reviewCount": "0",
    "bestRating": "5"
  }
}
```
> Note: Only add `aggregateRating` once real reviews exist. Google penalizes fake/empty ratings.

#### 4.3 VideoObject (when demo videos are created)

```json
{
  "@context": "https://schema.org",
  "@type": "VideoObject",
  "name": "ADI Quickstart - Deploy Your First AI Agent",
  "description": "Learn how to install ADI and deploy your first autonomous AI agent in under 5 minutes.",
  "thumbnailUrl": "https://adi.the-ihor.com/thumbnails/quickstart.png",
  "uploadDate": "2026-01-15",
  "duration": "PT5M",
  "contentUrl": "https://youtube.com/watch?v=...",
  "embedUrl": "https://youtube.com/embed/..."
}
```

#### 4.4 ItemList (for plugin registry)

```json
{
  "@context": "https://schema.org",
  "@type": "ItemList",
  "name": "ADI Plugins",
  "description": "Official and community plugins for ADI agent infrastructure",
  "numberOfItems": 15,
  "itemListElement": [
    {
      "@type": "ListItem",
      "position": 1,
      "item": {
        "@type": "SoftwareApplication",
        "name": "adi.agent-loop",
        "description": "Autonomous LLM agent orchestration plugin"
      }
    }
  ]
}
```

#### 4.5 Person (for author pages)

```json
{
  "@context": "https://schema.org",
  "@type": "Person",
  "name": "Ihor Herasymovych",
  "jobTitle": "Founder",
  "worksFor": { "@type": "Organization", "name": "ADI" },
  "url": "https://adi.the-ihor.com/en/authors/ihor",
  "sameAs": [
    "https://github.com/mgorunuch",
    "https://twitter.com/mgorunuch",
    "https://linkedin.com/in/mgorunuch"
  ]
}
```

#### 4.6 WebApplication (for the platform dashboard)

```json
{
  "@context": "https://schema.org",
  "@type": "WebApplication",
  "name": "ADI Platform",
  "url": "https://adi.the-ihor.com/en/dashboard",
  "applicationCategory": "DeveloperApplication",
  "operatingSystem": "Any",
  "offers": {
    "@type": "AggregateOffer",
    "lowPrice": "0",
    "highPrice": "99",
    "priceCurrency": "USD",
    "offerCount": "3"
  }
}
```

---

## 5. SERP Feature Targeting

### 5.1 Featured Snippets

Target "definition" and "how-to" queries with structured content.

**Strategy**: Place a concise definition (40-60 words) immediately after an H2/H3 matching the query, followed by a list or table.

**Target queries and content format:**

| Query | Snippet Type | Target Page |
|-------|-------------|-------------|
| "what is agent development infrastructure" | Paragraph | /docs (intro) |
| "how to deploy ai agents" | Numbered list | /docs/quickstart |
| "ai agent framework comparison" | Table | /comparisons |
| "what is a cocoon in ai" | Paragraph | /products/cocoon |
| "how to index code for llm" | Numbered list | /products/indexer |
| "typespec vs protobuf" | Table | /products/tsp-gen |

**Content template for paragraph snippets:**
```markdown
## What is Agent Development Infrastructure?

Agent Development Infrastructure (ADI) is a modular platform for building,
deploying, and orchestrating autonomous AI agents. It provides containerized
runtimes, semantic code indexing, persistent memory, and a plugin architecture
-- all built in Rust for maximum performance and reliability.
```

**Content template for list snippets:**
```markdown
## How to Deploy an AI Agent with ADI

1. Install ADI CLI: `curl -fsSL https://adi.the-ihor.com/install.sh | sh`
2. Initialize your project: `adi init`
3. Configure the agent loop in `adi.toml`
4. Start a Cocoon runtime: `adi cocoon start`
5. Deploy your agent: `adi agent run --cocoon <id>`
```

### 5.2 People Also Ask (PAA)

Create dedicated FAQ sections on key pages targeting these questions:

**Homepage / Docs FAQ:**
- What is ADI used for?
- Is ADI free to use?
- What programming languages does ADI support?
- How does ADI compare to LangChain?
- Can I self-host ADI?

**Cocoon FAQ:**
- What is a Cocoon in ADI?
- How do Cocoons isolate AI agent execution?
- Can I run multiple agents in one Cocoon?

**Pricing FAQ:**
- Is there a free tier for ADI?
- What's included in ADI Pro?
- Does ADI offer a team plan?

**Implementation**: Use `<details>` / `<summary>` or dedicated FAQ components with `FAQPage` schema markup on each.

### 5.3 Knowledge Panel

**Goal**: Trigger a Google Knowledge Panel for "ADI agent development infrastructure."

**Requirements:**
1. Consistent `Organization` schema across all pages (implemented)
2. Wikidata entry for ADI (to create)
3. CrunchBase profile (if applicable)
4. Consistent NAP (Name, Address, Phone) if physical presence exists
5. Google Business Profile (if applicable)
6. Strong branded search volume (build over time)
7. Wikipedia article (requires notability criteria -- long-term goal)

**Immediate actions:**
- Create Wikidata item for ADI
- Ensure GitHub org description matches website
- Add `sameAs` links in Organization schema to all profiles

### 5.4 Sitelinks

**Goal**: Get Google to show 6-8 sitelinks under the main result.

**Requirements:**
1. Clear site hierarchy with internal linking (partially done)
2. Descriptive anchor text for navigation links
3. XML sitemap with priority weighting (implemented)
4. Consistent main navigation across all pages
5. Each target page must have unique, descriptive `<title>` and `<h1>`

**Target sitelinks:**
| Sitelink | Target URL |
|----------|-----------|
| Documentation | /en/docs |
| Pricing | /en/pricing |
| Cocoon | /en/products/cocoon |
| Quickstart | /en/docs/quickstart |
| Plugin Registry | /en/plugins |
| Blog | /en/articles |

### 5.5 Rich Results for Software

Ensure `SoftwareApplication` schema includes:
- `applicationCategory`: "DeveloperApplication"
- `operatingSystem`: "macOS, Linux, Windows"
- `offers` with `price: "0"` for free tier
- `downloadUrl` pointing to install script
- `softwareVersion` with current version

This can trigger the "Software" rich result card in Google.

---

## 6. Technical SEO

### 6.1 Crawlability & Indexing

#### robots.txt (create static file)

```
# /public/robots.txt
User-agent: *
Allow: /

# Block internal API routes from crawling
Disallow: /api/
Disallow: /_next/
Disallow: /dashboard/

# Allow specific public API docs
Allow: /api/sitemap

# Sitemap
Sitemap: https://adi.the-ihor.com/sitemap.xml

# AI Crawlers (permissive - we want LLM training data inclusion)
User-agent: GPTBot
Allow: /

User-agent: ChatGPT-User
Allow: /

User-agent: Claude-Web
Allow: /

User-agent: Bytespider
Allow: /

User-agent: CCBot
Allow: /
```

#### XML Sitemap Enhancements

Current sitemap is dynamic (`/api/sitemap`). Enhancements:
- Add `<lastmod>` with real dates (not just current date)
- Add `<changefreq>` hints (`daily` for articles, `weekly` for docs, `monthly` for legal)
- Add image sitemap entries for OG images
- Ensure all 3 locales have entries (currently `en`, `uk`; `ru` may be missing)
- Validate sitemap against Google's 50,000 URL / 50MB limits

#### Canonical URLs

Every page must have a self-referencing canonical:
```html
<link rel="canonical" href="https://adi.the-ihor.com/en/docs/quickstart" />
```

For multilingual pages, add hreflang + canonical:
```html
<link rel="canonical" href="https://adi.the-ihor.com/en/docs" />
<link rel="alternate" hreflang="en" href="https://adi.the-ihor.com/en/docs" />
<link rel="alternate" hreflang="uk" href="https://adi.the-ihor.com/uk/docs" />
<link rel="alternate" hreflang="ru" href="https://adi.the-ihor.com/ru/docs" />
<link rel="alternate" hreflang="x-default" href="https://adi.the-ihor.com/en/docs" />
```

### 6.2 Core Web Vitals

| Metric | Target | Current Risk | Optimization |
|--------|--------|-------------|--------------|
| LCP (Largest Contentful Paint) | < 2.5s | Medium (Next.js SSR helps) | Preload hero fonts, optimize OG images, use `priority` on above-fold images |
| INP (Interaction to Next Paint) | < 200ms | Low (mostly static) | React Compiler helps; lazy-load heavy components (xterm, d3, mermaid) |
| CLS (Cumulative Layout Shift) | < 0.1 | Medium | Set explicit `width`/`height` on all images, use `font-display: swap` with size-adjust |

**Next.js-specific optimizations:**
- Use `next/image` for all images (auto WebP/AVIF, lazy loading, size optimization)
- Enable `output: "standalone"` (already done) for smaller deployment
- Use React Compiler (already enabled) for automatic memoization
- Implement `loading.tsx` skeletons for doc pages to prevent CLS
- Preconnect to external origins: `<link rel="preconnect" href="https://fonts.gstatic.com" />`

### 6.3 Page Speed Optimizations

- **Font loading**: Inter + JetBrains Mono via `next/font` (already optimal, self-hosted)
- **JavaScript**: Code-split heavy libraries (d3, mermaid, xterm) behind dynamic imports
- **CSS**: Tailwind v4 with purging (automatic in Next.js)
- **Images**: Convert all PNG/JPG to WebP. Use AVIF where supported. Serve responsive sizes.
- **Caching**: Set aggressive `Cache-Control` for static assets (`/_next/static/` -> `immutable, max-age=31536000`)

### 6.4 URL Structure

**Current**: `/{locale}/{section}/{slug}` -- Good.

**Rules:**
- All URLs lowercase, hyphens for word separation
- No trailing slashes (pick one convention and redirect the other)
- Max 3 levels deep: `/en/docs/libs/core` is the deepest acceptable
- Locale prefix required for all pages (`/en/`, `/uk/`, `/ru/`)
- Redirect bare `/docs` to `/en/docs` (default locale)

### 6.5 Internal Linking

**Strategy**: Every page should have 3-5 contextual internal links minimum.

**Link architecture:**
```
Homepage
├── /pricing (CTA from hero, features)
├── /products/* (from feature grid)
├── /docs (from hero CTA)
│   ├── /docs/quickstart (from intro)
│   ├── /docs/architecture (from concepts)
│   ├── /docs/agents (from quickstart)
│   ├── /docs/plugins (cross-linked from components)
│   └── /docs/cli (from all docs)
├── /articles/* (from homepage blog section)
│   └── Each article links to 2-3 related articles + relevant docs
└── /plugins (from docs, products)
```

**Orphan page check**: Ensure every page in the sitemap is reachable from at least 2 other pages via HTML links (not just sitemap).

### 6.6 Mobile SEO

- Responsive design via Tailwind (already implemented)
- Ensure touch targets are 48x48px minimum
- Test all interactive components (code blocks, terminal embeds) on mobile
- Mobile-first indexing: Google indexes the mobile version -- verify no content is hidden on mobile

### 6.7 International SEO (hreflang)

**Current locales**: en (default), uk (Ukrainian), ru (Russian)

**Issue found**: Middleware routes only `en` and `uk`, but locale config includes `ru`. This means Russian pages may not be routable. Fix the middleware to include all 3 locales.

**hreflang implementation checklist:**
- [ ] Every page has hreflang annotations for all locale variants
- [ ] `x-default` points to English version
- [ ] hreflang is reciprocal (en page links to uk, uk page links to en)
- [ ] Sitemap includes `<xhtml:link rel="alternate">` for all locales (partially implemented)
- [ ] Russian locale is accessible via middleware routing

---

## 7. Content Strategy

### 7.1 Content Gap Analysis

**Missing high-value pages:**

| Page | Target Keyword | Priority |
|------|---------------|----------|
| `/comparisons/adi-vs-langchain` | adi vs langchain | High |
| `/comparisons/adi-vs-crewai` | adi vs crewai | High |
| `/comparisons/adi-vs-autogen` | adi vs autogen | High |
| `/comparisons/adi-vs-dify` | adi vs dify | Medium |
| `/use-cases` | ai agent use cases | High |
| `/use-cases/code-review` | ai code review agent | Medium |
| `/use-cases/automated-testing` | ai testing agent | Medium |
| `/changelog` | adi changelog | Medium |
| `/integrations` | adi integrations | Medium |
| `/enterprise` | enterprise ai agents | Medium |
| `/security` | ai agent security | Medium |
| `/blog/category/tutorials` | ai agent tutorial | High |

### 7.2 Content Calendar Themes

**Monthly content pillars (rotating):**

| Month Theme | Content Types |
|-------------|---------------|
| Agent Patterns | Tutorial: building specific agent types, architecture patterns |
| Infrastructure Deep-Dives | Technical: Cocoon internals, plugin system, hive orchestration |
| Comparisons & Migrations | Comparison pages, migration guides from competing tools |
| Use Cases & Case Studies | Real-world deployments, customer stories, benchmarks |
| Developer Experience | CLI tips, workflow optimization, tooling guides |
| Security & Compliance | Sandboxing, secrets management, audit logging |

### 7.3 Blog Content Templates

**Tutorial post structure (optimized for featured snippets):**
```markdown
# [How to / Guide to] [Specific Outcome]

> **TL;DR**: [1-2 sentence summary -- targets featured snippet]

## Prerequisites
- [Bulleted list]

## Step 1: [Action]
[Explanation + code block]

## Step 2: [Action]
[Explanation + code block]

...

## Common Issues
### [Issue 1]
[Solution]

## Next Steps
- [Internal links to related content]

## FAQ
### [Question matching PAA]
[Concise answer]
```

**Comparison post structure:**
```markdown
# ADI vs [Competitor]: [Aspect] Comparison

> **Summary**: [2-3 sentences with verdict -- targets featured snippet]

## Quick Comparison

| Feature | ADI | [Competitor] |
|---------|-----|--------------|
| ...     | ... | ...          |

## [Feature 1] in Detail
### ADI
[Explanation]
### [Competitor]
[Explanation]

...

## When to Choose ADI
[Bulleted list]

## When to Choose [Competitor]
[Bulleted list -- being fair builds trust and E-E-A-T]

## Migration Guide
[If applicable]
```

### 7.4 E-E-A-T Signals (Experience, Expertise, Authoritativeness, Trustworthiness)

**Experience:**
- Include real code examples from the ADI codebase (not generic)
- Show terminal output / screenshots of actual usage
- Reference specific version numbers and commit hashes

**Expertise:**
- Author bios with credentials on all articles
- Link to author's GitHub contributions
- Technical depth: explain *why*, not just *how*

**Authoritativeness:**
- Link to GitHub repo (show stars, contributors)
- Reference benchmarks with reproducible methodology
- Get cited by other developer tools / blogs

**Trustworthiness:**
- BSL-1.1 license clearly explained
- Privacy policy and terms accessible
- HTTPS everywhere (already done)
- Contact information visible
- Display real usage stats when available (downloads, active users)

---

## 8. Link Building & Authority

### 8.1 Earned Link Opportunities

| Strategy | Target | Effort | Impact |
|----------|--------|--------|--------|
| GitHub README SEO | Own repos | Low | Medium |
| Awesome Lists | awesome-rust, awesome-ai-agents | Low | High |
| Dev.to / Hashnode cross-posting | Blog articles | Medium | Medium |
| Hacker News / Reddit launches | Product launches | Low | High (spiky) |
| Conference talks / demos | RustConf, AI Engineer Summit | High | High |
| Open-source integrations | MCP ecosystem, LLM tools | Medium | High |
| Developer tool directories | AlternativeTo, StackShare, Product Hunt | Low | Medium |

### 8.2 GitHub-Driven SEO

- Optimize GitHub org description: "Think agents. Think ADI. -- The agent company. Infrastructure for building, deploying, and orchestrating autonomous AI agents. Rust."
- Each repo README should link back to `adi.the-ihor.com` with descriptive anchor text
- Use GitHub Topics: `ai-agents`, `rust`, `llm`, `agent-infrastructure`, `mcp`, `developer-tools`
- GitHub Discussions / Wiki for community-driven long-tail content

### 8.3 Content Syndication

- Cross-post articles to Dev.to with `canonical_url` pointing to ADI
- Share technical posts on Hacker News, r/rust, r/MachineLearning
- Create Twitter/X threads summarizing key articles (@adi_agents)
- YouTube tutorials (triggers VideoObject rich results)

### 8.4 Directory Listings (High-DR Backlinks)

| Directory | URL | DR | Action |
|-----------|-----|------|--------|
| AlternativeTo | alternativeto.net | 80+ | List as alternative to LangChain, AutoGen |
| StackShare | stackshare.io | 85+ | Create company + tool profile |
| Product Hunt | producthunt.com | 90+ | Launch each standalone product separately |
| G2 | g2.com | 90+ | Create vendor profile (when reviews exist) |
| Awesome Rust | github.com/rust-unofficial/awesome-rust | 70+ | Submit PR for ADI |
| Awesome AI Agents | Various lists | 50-70 | Submit to all relevant lists |
| MCP Directory | Official MCP ecosystem | Varies | List browser-debug MCP server |
| Crates.io | crates.io | 80+ | Publish libraries with links to docs |

---

## 9. Current Implementation Audit

### Issues Found

| Issue | Severity | Location | Fix |
|-------|----------|----------|-----|
| No static `robots.txt` | High | `/public/` | Create with Disallow for `/api/`, `/dashboard/`, `/_next/` |
| `site.webmanifest` empty name/short_name | Medium | `/public/site.webmanifest` | Set `"name": "ADI"`, `"short_name": "ADI"` |
| Russian locale not routed | Medium | `middleware.ts` | Add `"ru"` to middleware locale list |
| No `<changefreq>` in sitemap | Low | `/api/sitemap` | Add frequency hints per page type |
| No `<lastmod>` with real dates | Medium | `/api/sitemap` | Track content update dates, use in sitemap |
| Missing OG images for most pages | Medium | Product/doc pages | Generate per-page OG images (use `next/og`) |
| No `robots.txt` file (only meta robots) | High | `/public/robots.txt` | Crawlers check `/robots.txt` first -- must exist as a file |
| Potential thin content on doc pages | High | `/docs/*` | Audit which doc pages have actual content vs. placeholder |
| No breadcrumb UI component | Low | All pages | Add visible breadcrumb navigation (complements schema) |
| Missing `rel="noopener"` on external links | Low | All pages | Add to all `target="_blank"` links |
| No 404 page optimization | Medium | `not-found.tsx` | Custom 404: "This page doesn't exist. But your agents can. Think ADI." + search, popular links |
| No redirect from `www.` subdomain | Medium | DNS/Traefik | 301 redirect `www.adi.the-ihor.com` to `adi.the-ihor.com` |

### What's Working Well

- Comprehensive structured data suite (Organization, Website, SoftwareApplication, Article, FAQ, HowTo, Breadcrumb)
- Dynamic sitemap with hreflang support
- GTM with GDPR-compliant consent management
- Self-hosted fonts via `next/font` (no render-blocking external requests)
- Proper title template (`%s | ADI`)
- OpenGraph + Twitter Card meta tags on all pages
- React Compiler enabled (better runtime performance)
- Standalone output mode (optimal deployment)
- Speakable specification on articles (voice search / LLM readiness)

---

## 10. Implementation Roadmap

### Phase 0: Positioning Rollout (Day 1-3)

- [ ] Update homepage hero: "Think agents. Think ADI." as primary headline
- [ ] Update root layout.tsx: title default to "ADI - The Agent Company | Agent Development Infrastructure"
- [ ] Update root layout.tsx: description to lead with "Think agents. Think ADI."
- [ ] Update OG image to include tagline "Think agents. Think ADI."
- [ ] Update `site.webmanifest`: name = "ADI - The Agent Company"
- [ ] Update GitHub org description with new positioning
- [ ] Update Twitter/X bio: "The agent company. Build, deploy, orchestrate. https://adi.the-ihor.com"
- [ ] Create `/public/llms.txt` with positioning-first copy
- [ ] Update Organization schema `description` to include "The agent company"
- [ ] Add tagline to CLI `--version` output: `adi x.y.z -- The agent company`

### Phase 1: Foundation (Week 1-2)

- [ ] Create static `/public/robots.txt` with proper directives
- [ ] Fix `site.webmanifest` (name, short_name, description, theme_color)
- [ ] Fix Russian locale routing in middleware
- [ ] Audit all doc pages for thin/missing content
- [ ] Add real `<lastmod>` dates to sitemap
- [ ] Verify canonical URLs on all pages
- [ ] Verify hreflang is reciprocal across all locale pairs
- [ ] Set up Google Search Console and submit sitemap
- [ ] Set up Bing Webmaster Tools

### Phase 2: Content & Rich Results (Week 3-4)

- [ ] Write meta titles + descriptions for all pages (use templates from Section 2)
- [ ] Add FAQ sections with `FAQPage` schema to: homepage, pricing, each product page
- [ ] Create comparison pages: ADI vs LangChain, ADI vs CrewAI, ADI vs AutoGen
- [ ] Add `SoftwareSourceCode` schema to docs/architecture page
- [ ] Add `Product` schema to pricing page (without fake ratings)
- [ ] Generate per-page OG images using `next/og` (ImageResponse API)
- [ ] Add visible breadcrumb UI component across all pages
- [ ] Optimize 404 page with search and popular links

### Phase 3: Content Expansion (Week 5-8)

- [ ] Write 4 tutorial articles targeting featured snippet queries
- [ ] Create `/use-cases` landing page + 3 use case subpages
- [ ] Create `/integrations` page listing all MCP, LLM, and tool integrations
- [ ] Create `/changelog` page (auto-generated from git tags or manual)
- [ ] Add `/security` page covering sandboxing, isolation, and data handling
- [ ] Fill in all doc pages with substantive content (min 500 words each)
- [ ] Internal linking audit: ensure every page has 3-5 contextual links

### Phase 4: Authority & Distribution (Week 9-12)

- [ ] Submit to Awesome Rust, Awesome AI Agents lists
- [ ] Create AlternativeTo, StackShare profiles
- [ ] Plan Product Hunt launch
- [ ] Cross-post 3 articles to Dev.to with canonical URLs
- [ ] Create Wikidata entry for ADI
- [ ] Publish 2 libraries to crates.io with doc links
- [ ] Optimize GitHub org + repo descriptions and topics

### Phase 5: Monitoring & Iteration (Ongoing)

- [ ] Weekly: Check Google Search Console for crawl errors, coverage issues
- [ ] Monthly: Review keyword rankings, identify new opportunities
- [ ] Monthly: Publish 2-4 new articles/tutorials
- [ ] Quarterly: Update comparison pages with latest competitor features
- [ ] Quarterly: Run Lighthouse CI audits, fix any regressions
- [ ] Quarterly: Review and update structured data for new Google features

---

## 11. Open & Community-First Messaging

### The Promise

> **ADI is built in the open. We don't lock you in, we don't rug-pull, we don't enshittify.**

This is the trust anchor. Every developer who evaluates ADI should understand within 30 seconds: this is a community project that respects its users.

### Why This Matters for SEO

Trust signals directly affect search rankings through E-E-A-T (Trustworthiness is the most weighted factor). Open-source and community-first projects earn:

1. **Natural backlinks** -- developers link to tools they trust, not tools they're trapped by
2. **Positive brand mentions** -- no "ADI alternatives" rage-posts to outrank you
3. **Community-generated content** -- contributors write tutorials, blog posts, and Stack Overflow answers
4. **Lower bounce rates** -- visitors who trust the project explore more pages
5. **Higher branded search volume** -- word-of-mouth drives "ADI agents" searches

### Core Commitments (Public-Facing)

These are concrete, verifiable claims -- not vague promises. Each one should appear on the website and be referenced in meta descriptions, FAQ schemas, and comparison pages.

| Commitment | What It Means | Where to Display |
|------------|---------------|-----------------|
| **Open source** | All core libraries are source-available under BSL-1.1. Read every line. | Homepage, footer, GitHub, docs intro |
| **No vendor lock-in** | Standard protocols (HTTP, WebSocket, MCP). Export your data. Bring your own LLM keys. | Pricing page, comparison pages, docs |
| **No bait-and-switch pricing** | Free tier stays free. No retroactive feature gating. Price changes never apply to existing plans. | Pricing page, FAQ |
| **Community-driven roadmap** | Public roadmap. GitHub Issues for feature requests. Community votes shape priorities. | Roadmap page, GitHub |
| **Offline-capable** | CLI and plugins work without phoning home. No telemetry without consent. | Docs, privacy policy, installation guide |
| **Interoperable** | Works with any OpenAI-compatible API, any MCP server, any Docker runtime. Not just our stack. | Docs, integration page |
| **Forkable** | If we disappear tomorrow, you can fork and run everything. No proprietary dependencies. | About page, license page |

### Voice for Open Messaging

| Do | Don't |
|----|-------|
| "Source-available. Read every line." | "We believe in transparency." (vague) |
| "Your data. Your keys. Your infra." | "We respect your data." (means nothing) |
| "No telemetry without consent." | "We take privacy seriously." (everyone says this) |
| "Fork it. Run it. We don't mind." | "Open-source friendly." (what does that mean) |
| "Free means free. No gotchas." | "Generous free tier." (implies they're doing you a favor) |
| "Standard protocols. No lock-in." | "Designed for flexibility." (empty) |

### SEO Implementation

#### Dedicated Pages to Create

| Page | URL | Target Keywords |
|------|-----|----------------|
| Open Source Philosophy | `/en/open` | open source ai agent platform, self-hosted ai agents |
| Privacy & Data | `/en/privacy` | ai agent privacy, no telemetry ai tools |
| License Explained | `/en/license` | bsl license ai, source available ai tools |
| Roadmap | `/en/roadmap` | adi roadmap, ai agent platform roadmap |

#### Meta Description Templates for Trust Pages

```
Open Source Philosophy:
Title:  Open by Default - ADI's Commitment to Developers | ADI
Desc:   Source-available. No vendor lock-in. No telemetry without consent.
        ADI is agent infrastructure you can read, fork, and self-host.
        Built by developers, for developers.

License:
Title:  ADI License - BSL-1.1 Explained in Plain English | ADI
Desc:   ADI uses the Business Source License 1.1. Free for individuals
        and small teams. Full source access. Read what it means for
        your project -- no lawyer required.
```

#### FAQ Schema for Trust Questions

Add to homepage and pricing page with `FAQPage` markup:

- **Is ADI open source?** -- ADI is source-available under BSL-1.1. All core libraries are on GitHub. You can read, build, and self-host the entire stack. Free for individuals and small teams.
- **Does ADI collect telemetry?** -- No. ADI CLI and plugins work fully offline. Analytics are opt-in and self-hosted. We never phone home without your explicit consent.
- **Can I self-host ADI?** -- Yes. Every component runs on your infrastructure. Docker Compose files included. No cloud dependency required.
- **What happens if ADI shuts down?** -- You keep running. All source code is public. No proprietary dependencies. Fork the repos and continue.
- **Will the free tier stay free?** -- Yes. We don't bait-and-switch. The free tier scope is defined and won't shrink. Price changes never retroactively affect existing users.
- **Does ADI lock me into specific LLM providers?** -- No. ADI works with any OpenAI-compatible API. Bring your own keys. Switch providers anytime. We proxy, we don't lock.

#### Comparison Page Angle

Every comparison page (ADI vs LangChain, ADI vs CrewAI, etc.) should include an "Openness" row in the comparison table:

```markdown
| Aspect | ADI | [Competitor] |
|--------|-----|--------------|
| Source access | Full source, BSL-1.1 | [Varies] |
| Self-hostable | Yes, all components | [Varies] |
| Telemetry | None without consent | [Varies] |
| Vendor lock-in | None -- standard protocols | [Varies] |
| Data portability | Full export, your infrastructure | [Varies] |
```

#### Content Integration

Weave the open/community message into existing content naturally:

- **Homepage hero subtext**: "Open infrastructure for autonomous agents."
- **Pricing page header**: "Honest pricing. No surprises."
- **Docs introduction**: "Everything here is source-available. Found a bug? Fix it and PR."
- **Installation page**: "Installs offline. No account required. No telemetry."
- **Blog posts**: End with "ADI is open. [Star us on GitHub](link) or [join the discussion](link)."
- **404 page**: "This page doesn't exist. But the source code does. [Read it on GitHub](link)."

### Community Trust Signals for Schema.org

Extend the `Organization` schema:

```json
{
  "@context": "https://schema.org",
  "@type": "Organization",
  "name": "ADI",
  "description": "The agent company. Open agent development infrastructure.",
  "ethicsPolicy": "https://adi.the-ihor.com/en/open",
  "publishingPrinciples": "https://adi.the-ihor.com/en/open",
  "knowsAbout": ["AI Agents", "Developer Tools", "Open Source Software"],
  "memberOf": {
    "@type": "Organization",
    "name": "Open Source Community"
  }
}
```

### Metrics to Track

| Signal | Measures | Target |
|--------|----------|--------|
| GitHub stars growth | Community adoption | Steady upward trend |
| "ADI" branded searches | Brand awareness from trust | Month-over-month increase |
| Backlinks from dev blogs | Organic trust endorsement | 10+ unique domains in 6 months |
| Bounce rate on `/open` | Message resonance | < 40% |
| Pricing page conversion | Trust → action | Track before/after adding trust messaging |
| "ADI alternative to" searches | Negative sentiment | Should remain low / near zero |

---

## Appendix A: Meta Tag Reference

### Required meta tags per page type

**All pages:**
```html
<title>{keyword} | ADI</title>
<meta name="description" content="{150-155 char description}" />
<link rel="canonical" href="{full URL}" />
<meta property="og:title" content="{title}" />
<meta property="og:description" content="{description}" />
<meta property="og:image" content="{1200x630 image URL}" />
<meta property="og:url" content="{canonical URL}" />
<meta property="og:type" content="website" />
<meta property="og:site_name" content="ADI" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:site" content="@adi_agents" />
<link rel="alternate" hreflang="en" href="{en URL}" />
<link rel="alternate" hreflang="uk" href="{uk URL}" />
<link rel="alternate" hreflang="ru" href="{ru URL}" />
<link rel="alternate" hreflang="x-default" href="{en URL}" />
```

**Article pages (additional):**
```html
<meta property="article:published_time" content="{ISO 8601}" />
<meta property="article:modified_time" content="{ISO 8601}" />
<meta property="article:author" content="{author URL}" />
<meta property="article:section" content="{category}" />
<meta property="article:tag" content="{tag1}" />
```

**Product pages (additional):**
```html
<meta property="product:price:amount" content="{price}" />
<meta property="product:price:currency" content="USD" />
```

## Appendix B: LLM/AI Crawler Optimization

Modern search increasingly involves LLM-based answer engines (Google AI Overviews, Perplexity, ChatGPT Browse). Optimize for these:

### llms.txt Standard

Create `/public/llms.txt` (emerging convention):
```
# ADI -- The Agent Company

> Think agents. Think ADI.

## About
ADI is the agent company. Modular Rust infrastructure for building,
deploying, and orchestrating autonomous AI agents. BSL-1.1 license
(free for individuals and small teams).

## Key Products
- Cocoon: Secure containerized AI agent runtime
- Indexer: Semantic code understanding with embeddings
- Knowledgebase: Graph-based persistent agent memory
- tsp-gen: Pure Rust TypeSpec code generator
- Analytics: Real-time agent metrics and dashboards

## Links
- Website: https://adi.the-ihor.com
- Documentation: https://adi.the-ihor.com/en/docs
- GitHub: https://github.com/adi-family
- Pricing: https://adi.the-ihor.com/en/pricing
- Install: curl -fsSL https://adi.the-ihor.com/install.sh | sh
```

### Content Structure for LLM Extraction

- Use semantic HTML (`<article>`, `<section>`, `<nav>`, `<aside>`)
- Place the most important information in the first 2 paragraphs
- Use clear H2/H3 hierarchy (LLMs parse headings as topic boundaries)
- Include structured data (LLMs increasingly consume JSON-LD)
- Avoid content behind JS-only rendering (use SSR/SSG -- Next.js does this)
- Include the `speakable` property in Article schema (already done)

### AI Overview Optimization

Google AI Overviews pull from pages that:
1. Directly answer the query in the first paragraph
2. Provide structured supporting evidence (lists, tables)
3. Have strong E-E-A-T signals
4. Are crawlable and indexable
5. Use clear, factual language (not marketing fluff)

Format key pages so the first paragraph below each H2 is a self-contained answer.
