---
page: "/for/enterprise"
title: "For Enterprise"
purpose: "Address enterprise requirements: security, compliance, support, and scale"

audiences:
  - id: "ciso"
    label: "CISO / Security lead"
    share: 30
    arrives_from: ["Security review process", "vendor evaluation"]
    intent: "Verify ADI meets security and compliance requirements"
    expectations:
      - "SOC 2 / ISO 27001 status"
      - "Data residency and processing guarantees"
      - "Self-hosted deployment — code never leaves their network"
      - "Audit logging"
    frustrations:
      - "No security documentation"
      - "Data sent to third-party APIs without control"
      - "No on-prem option"
    success: "Approves ADI for security review"

  - id: "vp-eng"
    label: "VP Engineering / CTO"
    share: 40
    arrives_from: ["Homepage persona selector", "board-level AI initiative"]
    intent: "Evaluate ADI as strategic AI tooling investment"
    expectations:
      - "Scale story: 100+ developers"
      - "SSO / SAML integration"
      - "Dedicated support and SLAs"
      - "Deployment flexibility (cloud, on-prem, hybrid)"
    frustrations:
      - "No enterprise features"
      - "Startup-grade reliability"
      - "No migration path from existing tools"
    success: "Requests enterprise demo or pilot program"

  - id: "procurement"
    label: "IT Procurement"
    share: 30
    arrives_from: ["Internal referral from engineering"]
    intent: "Understand licensing, pricing, and vendor terms"
    expectations:
      - "Annual licensing options"
      - "Volume discounts"
      - "Standard contract terms"
      - "Vendor stability indicators"
    frustrations:
      - "No clear enterprise pricing"
      - "No procurement-friendly documentation"
    success: "Initiates procurement process"

flow:
  - step: "See hero"
    sees: "Headline about secure, self-hosted AI infrastructure"
  - step: "Security section"
    sees: "Compliance badges, data flow diagram, audit features"
  - step: "Features"
    sees: "SSO, RBAC, audit logs, deployment options"
  - step: "Scale"
    sees: "Architecture for 100+ developers"
  - step: "CTA"
    sees: "Contact sales, schedule demo, download security whitepaper"
---

Enterprise buyers need trust signals, not feature lists. Lead with security and compliance. Show the deployment architecture that keeps code on their network. Make the procurement path frictionless. This page will be reviewed by non-technical stakeholders — keep language accessible but precise.
