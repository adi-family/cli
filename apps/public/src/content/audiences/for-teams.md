---
page: "/for/teams"
title: "For Engineering Teams"
purpose: "Show team leads and engineering managers how ADI scales from individual to team use"

audiences:
  - id: "team-lead"
    label: "Engineering team lead"
    share: 45
    arrives_from: ["Homepage persona selector", "colleague referral"]
    intent: "Evaluate ADI for their 5-50 person engineering team"
    expectations:
      - "Shared configuration and workflows"
      - "Usage analytics and insights"
      - "Centralized API key management"
      - "Clear pricing per seat"
    frustrations:
      - "No team features — just individual tool multiplied"
      - "No visibility into what agents are doing"
      - "Unclear pricing"
    success: "Starts a team trial or contacts for demo"

  - id: "devops-eng"
    label: "DevOps / Platform engineer"
    share: 30
    arrives_from: ["Google 'AI agent infrastructure team'"]
    intent: "Understand deployment and integration story"
    expectations:
      - "Self-hosted option or clear cloud architecture"
      - "CI/CD integration"
      - "Container orchestration (Hive/Cocoon)"
    frustrations:
      - "Cloud-only with no self-host path"
      - "No API or automation hooks"
    success: "Sees clear deployment path and tries integration"

  - id: "eng-manager"
    label: "Engineering manager"
    share: 25
    arrives_from: ["Industry report", "conference talk"]
    intent: "Justify AI tooling investment to leadership"
    expectations:
      - "ROI indicators (time saved, velocity metrics)"
      - "Security and compliance posture"
      - "Adoption path that doesn't disrupt existing workflows"
    frustrations:
      - "No business case support"
      - "No security documentation"
    success: "Has enough material to pitch internally"

flow:
  - step: "See hero"
    sees: "Headline about team productivity and shared infrastructure"
  - step: "Features grid"
    sees: "Team-specific features: shared config, analytics, key management"
  - step: "How it works"
    sees: "Diagram showing team setup and agent orchestration"
  - step: "Pricing"
    sees: "Clear per-seat pricing with free tier"
  - step: "CTA"
    sees: "Start team trial, contact sales, or self-host docs"
---

This page bridges individual developer value to team scale. The audience already knows what AI coding tools do — they need to understand the team story: shared workflows, centralized keys, usage analytics, and the path from "one dev tried it" to "whole team uses it." Lead with outcomes (velocity, consistency), support with features.
