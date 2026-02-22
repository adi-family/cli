---
page: "/"
title: "Homepage"
purpose: "Route visitors to the right persona path as fast as possible"

audiences:
  - id: "first-time-visitor"
    label: "First-time visitor"
    share: 60
    arrives_from: ["Google search", "Hacker News", "GitHub README", "social link"]
    intent: "Understand what ADI is in under 10 seconds"
    expectations:
      - "Clear one-liner explaining the product"
      - "Visual cues that this is a dev tool, not enterprise SaaS fluff"
      - "Fast path to the thing they care about"
    frustrations:
      - "Walls of marketing text"
      - "No clear 'what is this?' above the fold"
      - "Forced sign-up before seeing value"
    success: "Clicks a persona path within 15 seconds"

  - id: "returning-user"
    label: "Returning user"
    share: 25
    arrives_from: ["Bookmark", "direct URL", "CLI link"]
    intent: "Navigate to docs, dashboard, or download"
    expectations:
      - "Persistent nav with docs/github links"
      - "Quick access to the path they chose last time"
    frustrations:
      - "Having to re-select persona every visit"
    success: "Reaches their destination in one click"

  - id: "evaluator"
    label: "Technical evaluator"
    share: 15
    arrives_from: ["Comparison article", "colleague recommendation"]
    intent: "Assess if ADI fits their stack and workflow"
    expectations:
      - "Architecture overview or link to one"
      - "Clear differentiation from competitors"
      - "Open source signals (GitHub stars, license)"
    frustrations:
      - "No technical depth on landing page"
      - "Hidden pricing"
    success: "Finds enough info to continue to a persona page or docs"

flow:
  - step: "Land on page"
    sees: "Hero with 'who are you?' prompt"
  - step: "Scan options"
    sees: "5 persona cards with short descriptions"
  - step: "Click persona"
    goes_to: "/for/{persona}"
---

The homepage is a router, not a pitch. It asks one question — "who are you?" — and sends the visitor to a tailored experience. Keep it minimal: badge, headline, persona selector, footnote. No features grid, no testimonials, no pricing. Those live on persona pages.
