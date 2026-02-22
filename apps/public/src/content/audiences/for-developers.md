---
page: "/for/developers"
title: "For Individual Developers"
purpose: "Convince a solo developer to install the CLI and start using ADI today"

audiences:
  - id: "indie-dev"
    label: "Independent developer"
    share: 50
    arrives_from: ["Homepage persona selector", "Google 'AI coding assistant CLI'"]
    intent: "Find a free, local-first AI dev tool they control"
    expectations:
      - "One-line install command (curl/brew)"
      - "BYOK — bring your own API keys, no vendor lock-in"
      - "Works offline or with local models"
      - "Clear 'free forever for individuals' messaging"
    frustrations:
      - "Tools that require cloud accounts to function"
      - "Subscription walls for basic features"
      - "Closed-source agents they can't inspect"
    success: "Copies install command and runs it"

  - id: "power-user"
    label: "Power user / tinkerer"
    share: 30
    arrives_from: ["GitHub README", "Rust community"]
    intent: "Understand the plugin system and extend ADI"
    expectations:
      - "Plugin SDK docs or link"
      - "Architecture overview — what's a crate, what's a plugin"
      - "Source code access"
    frustrations:
      - "No extension points"
      - "Monolithic black-box design"
    success: "Stars the repo and starts reading plugin SDK docs"

  - id: "comparison-shopper"
    label: "Comparing alternatives"
    share: 20
    arrives_from: ["'ADI vs Cursor' search", "comparison blog"]
    intent: "See what makes ADI different from Copilot/Cursor/Aider"
    expectations:
      - "Honest feature comparison"
      - "Clear positioning: local-first, Rust, plugin architecture"
      - "Not a clone of existing tools"
    frustrations:
      - "Vague 'AI-powered' claims"
      - "No differentiation"
    success: "Understands ADI's unique value and tries it"

flow:
  - step: "See hero"
    sees: "Headline: free, local-first, your keys"
  - step: "Install"
    sees: "Copy-paste install command"
  - step: "Features"
    sees: "3-4 key capabilities with short descriptions"
  - step: "How it works"
    sees: "Terminal demo or architecture diagram"
  - step: "CTA"
    sees: "Install + docs + GitHub links"
---

This page sells the developer experience. Lead with the install command — developers want to try, not read. Show that it's free, local, and extensible. The page should feel like a well-written README, not a marketing site. Terminal screenshots > stock photos. Code examples > bullet points.
