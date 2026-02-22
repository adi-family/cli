import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

const footerLinks = [
  { key: "privacy" as const, href: "/privacy" as const },
  { key: "terms" as const, href: "/terms" as const },
];

export function Footer() {
  const t = useTranslations("nav");
  const tFooter = useTranslations("footer");

  return (
    <footer className="border-t border-border">
      <div className="mx-auto flex max-w-[1200px] flex-col items-center justify-between gap-6 px-6 py-12 md:flex-row md:px-16">
        {/* Brand */}
        <div className="flex items-center gap-2">
          <svg viewBox="0 0 100 100" className="h-4 w-4 text-accent" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="20" y="20" width="60" height="60" rx="4" />
          </svg>
          <span className="text-sm text-text-muted">
            {tFooter("copyright", {year: new Date().getFullYear()})}
          </span>
        </div>

        {/* Links */}
        <nav className="flex flex-wrap items-center gap-6">
          {footerLinks.map((link) => (
              <Link
                key={link.key}
                href={link.href}
                className="text-sm text-text-muted hover:text-text transition-colors"
              >
                {t(link.key)}
              </Link>
            ),
          )}
        </nav>
      </div>
    </footer>
  );
}
