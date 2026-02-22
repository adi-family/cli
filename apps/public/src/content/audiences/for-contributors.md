---
page: "/for/contributors"
title: "For Open Source Contributors"
purpose: "Welcome contributors and make it trivially easy to start contributing"

audiences:
  - id: "rust-dev"
    label: "Rust developer"
    share: 40
    arrives_from: ["GitHub explore", "Rust community", "crates.io"]
    intent: "Find an interesting Rust project to contribute to"
    expectations:
      - "Clear crate structure and architecture"
      - "Good first issues labeled"
      - "Contribution guide"
      - "30+ crates = lots of entry points"
    frustrations:
      - "Monolithic codebase with no clear boundaries"
      - "No contribution guide"
      - "Maintainers don't respond to PRs"
    success: "Clones repo and picks a first issue"

  - id: "plugin-author"
    label: "Plugin / extension author"
    share: 35
    arrives_from: ["Plugin SDK docs", "community forum"]
    intent: "Build a plugin for ADI's ecosystem"
    expectations:
      - "Plugin SDK documentation"
      - "Example plugins to study"
      - "Plugin registry for distribution"
      - "Stable ABI guarantees"
    frustrations:
      - "Unstable plugin API"
      - "No examples"
      - "No distribution mechanism"
    success: "Builds and publishes their first plugin"

  - id: "community-member"
    label: "Community participant"
    share: 25
    arrives_from: ["Discord/forum", "blog post"]
    intent: "Participate in discussions, report bugs, suggest features"
    expectations:
      - "Active community channels"
      - "Roadmap visibility"
      - "Recognition for contributions"
    frustrations:
      - "Dead community channels"
      - "No roadmap"
    success: "Joins community and makes first contribution"

flow:
  - step: "See hero"
    sees: "Headline: 30+ Rust crates, source-available, come build with us"
  - step: "Architecture"
    sees: "Crate map showing all components and their relationships"
  - step: "Getting started"
    sees: "Clone, build, run tests in 3 commands"
  - step: "Plugin SDK"
    sees: "Quick intro to building plugins"
  - step: "CTA"
    sees: "GitHub, good first issues, community links"
---

This page is for builders who want to contribute, not consume. Lead with the architecture — show the 30+ crate ecosystem and where contributions fit. Make the first PR achievable in under an hour. Show that this is a real open-source project with active maintainers, not a corporate "open source" dump.
