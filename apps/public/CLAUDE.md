public-site, nextjs, seo, tailwind

## Build Rules
- NEVER run `npm run build` or `next build` — builds are slow and run in CI
- Only `npx tsc --noEmit` is allowed for type checking

## Overview
- Public-facing website for ADI at adi.the-ihor.com
- Next.js 15 App Router with Tailwind CSS 4
- ADID design system: dark-dominant, line-driven, purple accent (#875fd7)

## Architecture
- `src/lib/site.ts` — Site config (URLs, metadata)
- `src/components/layout/` — Header (glass morphism), Footer
- `src/components/seo/` — JSON-LD structured data (Organization, Website, Breadcrumb)
- `src/components/ui/` — TagChip, Breadcrumbs

## Routes
| Route | Description |
|-------|-------------|
| `/` | Homepage with hero + feature boxes |
| `/api/og` | Dynamic OG image endpoint |

## SEO Features
- JSON-LD: Organization, WebSite, BreadcrumbList
- Dynamic OG images (Edge runtime, 1200x630)
- robots.txt allowing AI crawlers (GPTBot, Claude-Web, etc.)
- Full OpenGraph + Twitter card metadata per page
- Canonical URLs

## Environment Variables
| Variable | Description |
|----------|-------------|
| `NEXT_PUBLIC_SITE_URL` | Public site URL (default: https://adi.the-ihor.com) |

## Theme
- Imports `packages/theme/generated/adi-theme.css` for design tokens
- Tailwind classes: `bg-bg`, `bg-surface`, `text-text`, `text-accent`, `border-border`
- Font stacks: `font-heading` (Space Grotesk), `font-body` (Inter), `font-mono` (JetBrains Mono)
