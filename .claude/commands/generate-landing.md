---
allowed-tools: Bash(ls:*), Bash(cat:*), Bash(git:*), Read, Grep, Glob, Edit, Write, Task
argument-hint: [path to project] [url]
description: Generate a landing page
---

Here is the repository with the info, read README file:
`$1`

Here is the location where to generate or update the landing page: `apps/infra-service-web/src/app/docs/$2`

## Landing Page Philosophy: Developer-Focused Documentation That Sells

**Target Audience:** Developers who value their time, hate marketing fluff — but are still human.

### The Truth About Developer Emotions

Developers aren't robots parsing specs. They're people who:
- **Feel frustrated** when tools waste their time
- **Feel relief** when something just works
- **Feel pride** when their stack is elegant
- **Feel joy** when they ship on Friday instead of Sunday
- **Feel trust** when someone respects their intelligence

**Your job:** Tap into real emotions through authentic value, not manufactured hype.

### Core Principles

1. **Lead with the Pain, Then the Relief**
   - Name the frustration they already feel
   - Show the moment it disappears
   - "You know that feeling when..." → "Now imagine..."

2. **Documentation IS Marketing**
   - Great docs are the best sales pitch for developers
   - If the docs are clear, the product is probably good
   - Every example should be copy-pasteable and working

3. **Respect Intelligence, Connect to Humanity**
   - No buzzwords ("revolutionary", "game-changing", "AI-powered")
   - No vague claims ("blazing fast") — use benchmarks
   - Yes to: "We built this because we were tired of X too"

### Emotional Triggers That Actually Work

| Emotion | How to Trigger (Authentically) |
|---------|-------------------------------|
| **Frustration → Relief** | "Tired of writing 50 lines just to...? One line." |
| **Confusion → Clarity** | Clear before/after. Show the mess they have vs clean solution. |
| **Anxiety → Confidence** | "Battle-tested in production at X scale" with real numbers. |
| **FOMO → Belonging** | "Join 10k developers who..." (only if true, with proof) |
| **Pride → Identity** | "For developers who care about..." (craftsmanship, performance, simplicity) |
| **Loneliness → Community** | Active Discord/GitHub with real conversations, not tumbleweeds |

### The Voice: Talk Like a Developer Friend

Not corporate. Not salesy. Like a senior dev recommending a tool over coffee:

```
❌ "Leverage our cutting-edge solution to maximize productivity"
✅ "I was mass-renaming files with sed and kept screwing up. Built this instead."

❌ "Enterprise-grade reliability"
✅ "We've been running this in prod for 2 years. Here's our uptime: 99.97%"

❌ "Seamless integration"
✅ "npm install, add 3 lines, done. Here's the diff."
```

### Structure That Sells

```
1. HOOK (5 seconds)
   - One-line problem statement
   - One-line solution
   - Install command (immediate action)

2. SHOW DON'T TELL (30 seconds)
   - Working code example
   - Before/after comparison
   - Real output/result

3. WHY THIS? (1 minute)
   - Honest comparison with alternatives
   - Clear trade-offs (what it's NOT good for)
   - Performance metrics with methodology

4. DEEP DIVE (for the convinced)
   - Architecture overview
   - API reference
   - Advanced examples
```

### Writing Style for Developers

- **Concise:** Every word must earn its place
- **Technical:** Use correct terminology, don't dumb down
- **Honest:** Acknowledge limitations upfront
- **Scannable:** Headers, bullets, code blocks — no walls of text

### What Makes Developers Trust (and Buy)

| Do | Don't |
|----|-------|
| Show the source code | Hide implementation details |
| Provide benchmarks with methodology | Say "fast" without numbers |
| List known issues/limitations | Pretend it's perfect |
| Compare honestly with competitors | Trash-talk alternatives |
| Offer free tier or trial | Gate everything behind sales calls |
| Active GitHub/Discord community | "Contact sales for support" |

### Call to Action That Works

```bash
# Bad: "Schedule a demo with our sales team"
# Good:
npm install awesome-lib
# or
curl -fsSL https://example.com/install.sh | sh
```

**The best CTA for developers is a working install command.**

### Example Hero Section (Emotional + Technical)

```markdown
# lib-name

You've mass-renamed files with sed. You've broken prod with a typo.
You've spent Sunday fixing what should've taken 5 minutes.

Never again.

\`\`\`bash
cargo add lib-name
\`\`\`

\`\`\`rust
// Before: Mass file operations with sed + find + xargs + prayer
// After:
lib_name::rename_all("src/**/*.ts", |name| name.to_snake_case());
// Done. No broken imports. No missed files. No Sunday.
\`\`\`

Built by developers who mass-renamed one too many times.
```

### Storytelling That Resonates

Every great developer tool has an origin story. Use yours:

1. **The Frustration Moment** — "We were migrating 500 endpoints when..."
2. **The Failed Attempts** — "We tried X, Y, Z. All sucked because..."
3. **The Insight** — "Then we realized if we just..."
4. **The Result** — "Now we do in 5 minutes what took 2 days"

This isn't manipulation. This is honesty. Developers connect with "I built this to scratch my own itch" — it's the most trusted origin story in software.

### Checklist Before Publishing

**Technical:**
- [ ] Can a developer understand the value in 10 seconds?
- [ ] Is there a working code example above the fold?
- [ ] Are install instructions copy-pasteable?
- [ ] Are there honest comparisons with alternatives?
- [ ] Are limitations clearly documented?
- [ ] Is there a path to try it for free?

**Emotional:**
- [ ] Does it name a pain they actually feel?
- [ ] Is there a moment of "oh, that's me" recognition?
- [ ] Does the voice feel human, not corporate?
- [ ] Is there a story (origin, why we built this)?
- [ ] Does it make them feel smart, not stupid?
- [ ] Is there community they can join?

**Both:**
- [ ] Would YOU be excited to find this page?
- [ ] Does it look good on mobile? (devs browse on phones too)


### Technical requirements

- if you need to use table, use at as html, not in markdown format

### Internationalization (i18n)

The web app uses **Fluent** for translations with English (en) and Ukrainian (uk) support.

**For UI strings (header, footer, buttons, labels):**

1. Add translation keys to `.ftl` files:
   ```
   src/locales/en/common.ftl  # Common UI strings
   src/locales/en/home.ftl    # Homepage strings
   src/locales/uk/common.ftl  # Ukrainian translations
   src/locales/uk/home.ftl
   ```

2. Use translations in client components:
   ```tsx
   "use client";
   import { useTranslation } from "@/lib/i18n";

   function MyComponent() {
     const { t } = useTranslation();
     return <h1>{t("my-key")}</h1>;
   }
   ```

**For docs/content pages (full page translation):**

1. Create MDX files per locale:
   ```
   src/content/docs/[page]/en.mdx
   src/content/docs/[page]/uk.mdx
   ```

2. Create a page.tsx that loads the correct locale:
   ```tsx
   import { type Locale } from "@/lib/i18n";
   import { getLocale } from "@/lib/i18n/server";

   const contentMap: Record<Locale, () => Promise<{ default: React.ComponentType }>> = {
     en: () => import("@/content/docs/[page]/en.mdx"),
     uk: () => import("@/content/docs/[page]/uk.mdx"),
   };

   export default async function Page() {
     const locale = await getLocale();
     const Content = (await contentMap[locale]()).default;
     return <Content />;
   }
   ```

**Key files:**
- `src/lib/i18n/` - i18n infrastructure
- `src/locales/{en,uk}/` - Fluent translation files (.ftl)
- `src/content/` - Localized MDX content

**When generating landing pages:**
- For simple pages: use translation keys with `useTranslation()`
- For content-heavy pages: create separate MDX files per locale
- Always provide both English and Ukrainian translations
