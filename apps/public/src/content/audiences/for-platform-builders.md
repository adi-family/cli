---
page: "/for/platform-builders"
title: "For Platform Builders"
purpose: "Show how to embed ADI's agent runtime into third-party products"

audiences:
  - id: "saas-founder"
    label: "SaaS founder / product lead"
    share: 40
    arrives_from: ["Google 'embed AI agent runtime'", "API marketplace"]
    intent: "Add AI agent capabilities to their product without building from scratch"
    expectations:
      - "Embeddable runtime with clear API surface"
      - "Licensing terms for commercial embedding"
      - "White-label or headless mode"
      - "Performance and resource requirements"
    frustrations:
      - "No embedding API — only end-user CLI"
      - "Restrictive licensing"
      - "Heavy runtime requirements"
    success: "Starts integration with ADI runtime"

  - id: "infra-engineer"
    label: "Infrastructure engineer"
    share: 35
    arrives_from: ["Technical blog", "Rust ecosystem"]
    intent: "Evaluate ADI crates as building blocks for their platform"
    expectations:
      - "Individual crate documentation"
      - "Stable API versioning"
      - "Minimal dependency footprint"
      - "Docker/container deployment story"
    frustrations:
      - "All-or-nothing — can't use individual crates"
      - "Unstable APIs"
      - "Poor documentation"
    success: "Integrates one or more ADI crates"

  - id: "dev-tool-maker"
    label: "Developer tool maker"
    share: 25
    arrives_from: ["Dev tool community", "conference"]
    intent: "Build complementary tooling on ADI's platform"
    expectations:
      - "MCP server support"
      - "Plugin SDK for deep integration"
      - "Marketplace or registry for distribution"
    frustrations:
      - "Closed ecosystem"
      - "No third-party integration points"
    success: "Builds a tool that integrates with ADI"

flow:
  - step: "See hero"
    sees: "Headline: embed agent intelligence into your product"
  - step: "Architecture"
    sees: "Runtime architecture, API surface, integration points"
  - step: "Use cases"
    sees: "2-3 examples of platform integrations"
  - step: "Crate catalog"
    sees: "Available crates with their purposes and stability level"
  - step: "CTA"
    sees: "API docs, integration guide, partnership inquiry"
---

Platform builders think in APIs, crates, and integration surfaces — not features. Show the architecture first, then the specific crates they can embed. Be clear about licensing for commercial use. This audience will evaluate code quality, API stability, and documentation depth before committing. Every claim should be backed by a link to actual code or docs.
