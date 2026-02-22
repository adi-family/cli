import fs from "fs";
import path from "path";
import matter from "gray-matter";

// --- Types ---

export type Audience = {
  id: string;
  label: string;
  share: number;
  arrives_from: string[];
  intent: string;
  expectations: string[];
  frustrations: string[];
  success: string;
};

export type FlowStep = {
  step: string;
  sees?: string;
  goes_to?: string;
};

export type AudienceData = {
  page: string;
  title: string;
  purpose: string;
  audiences: Audience[];
  flow: FlowStep[];
  body: string;
};

// --- Slug mapping ---

const PERSONA_SLUGS: Record<string, string> = {
  developers: "for-developers",
  teams: "for-teams",
  enterprise: "for-enterprise",
  contributors: "for-contributors",
  "platform-builders": "for-platform-builders",
};

const CONTENT_DIR = path.join(process.cwd(), "src/content/audiences");

// --- Loaders ---

function parseFile(filePath: string): AudienceData {
  const raw = fs.readFileSync(filePath, "utf-8");
  const { data, content } = matter(raw);

  return {
    page: data.page,
    title: data.title,
    purpose: data.purpose,
    audiences: data.audiences ?? [],
    flow: data.flow ?? [],
    body: content.trim(),
  };
}

/** Load audience data for a persona page slug (e.g. "developers"). */
export function getAudienceByPersona(persona: string): AudienceData | null {
  const filename = PERSONA_SLUGS[persona];
  if (!filename) return null;

  const filePath = path.join(CONTENT_DIR, `${filename}.md`);
  if (!fs.existsSync(filePath)) return null;

  return parseFile(filePath);
}

/** Load audience data by direct filename (e.g. "home"). */
export function getAudienceBySlug(slug: string): AudienceData | null {
  const filePath = path.join(CONTENT_DIR, `${slug}.md`);
  if (!fs.existsSync(filePath)) return null;

  return parseFile(filePath);
}

/** List all valid persona slugs for static generation. */
export function getAllPersonaSlugs(): string[] {
  return Object.keys(PERSONA_SLUGS);
}

/** List all audience files. */
export function getAllAudiences(): AudienceData[] {
  if (!fs.existsSync(CONTENT_DIR)) return [];

  return fs
    .readdirSync(CONTENT_DIR)
    .filter((f) => f.endsWith(".md"))
    .map((f) => parseFile(path.join(CONTENT_DIR, f)));
}
