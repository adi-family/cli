"use client";

import { useCallback, useEffect, useState } from "react";
import { useLocale, useTranslations } from "next-intl";
import { Link, usePathname, useRouter } from "@/i18n/navigation";
import { siteConfig } from "@/lib/site";
import { locales, localeLabels, type Locale } from "@/i18n/routing";

const APP_BASE_URL = "https://app.adi.local";

const navItems = [
  { key: "home" as const, href: "/" as const },
  { key: "app" as const, href: APP_BASE_URL, external: true },
];

const localeFullNames: Record<Locale, string> = {
  en: "English",
  uk: "Ukrainian",
  zh: "Chinese",
  de: "German",
  ru: "Russian",
};

export function Header() {
  const t = useTranslations("nav");
  const locale = useLocale() as Locale;
  const pathname = usePathname();
  const router = useRouter();
  const [mobileOpen, setMobileOpen] = useState(false);
  const [langModalOpen, setLangModalOpen] = useState(false);

  const switchLocale = (next: Locale) => {
    router.replace(pathname, { locale: next });
    setLangModalOpen(false);
    setMobileOpen(false);
  };

  const closeLangModal = useCallback(() => setLangModalOpen(false), []);

  useEffect(() => {
    if (!langModalOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeLangModal();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [langModalOpen, closeLangModal]);

  return (
    <>
      <header className="glass sticky top-0 z-50 border-b border-border">
        <div className="mx-auto flex h-l-4 max-w-[1200px] items-center justify-between p-h-15">
          {/* Logo */}
          <Link href="/" className="flex items-center g-075 group">
            <div className="flex s-2 items-center justify-center r-lg rounded border border-border group-hover:border-accent/40 transition-colors">
              <svg viewBox="0 0 100 100" className="s-1" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="20" y="20" width="60" height="60" rx="4" className="text-accent" />
              </svg>
            </div>
            <span className="font-heading text-lg font-semibold text-text tracking-tight">
              {siteConfig.name}
            </span>
          </Link>

          {/* Desktop nav */}
          <nav className="hidden md:flex items-center g-05">
            <div className="flex items-center r-pill rounded-p-025 border border-border bg-surface/50">
              {navItems.map((item) => {
                const active = !item.external && (item.href === "/" ? pathname === "/" : pathname.startsWith(item.href));
                const cls = "r-pill rounded p-v-05 p-h-1 text-sm text-text-muted transition-colors hover:text-text";
                return item.external ? (
                  <a
                    key={item.key}
                    href={`${item.href}/${locale}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className={cls}
                  >
                    {t(item.key)}
                  </a>
                ) : (
                  <Link
                    key={item.key}
                    href={item.href}
                    className={`${cls} ${active ? "bg-surface-alt !text-text" : ""}`}
                  >
                    {t(item.key)}
                  </Link>
                );
              })}
            </div>

            {/* Language selector trigger */}
            <button
              onClick={() => setLangModalOpen(true)}
              className="flex items-center g-025 r-pill rounded p-v-05 p-h-075 border border-border bg-surface/50 text-sm text-text-muted transition-colors hover:text-text hover:border-accent/40"
              aria-label="Select language"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="icon-sm">
                <circle cx="12" cy="12" r="10" />
                <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10A15.3 15.3 0 0 1 12 2z" />
              </svg>
              {localeLabels[locale]}
            </button>
          </nav>

          {/* Mobile burger */}
          <button
            onClick={() => setMobileOpen(!mobileOpen)}
            className="flex md:hidden s-2 items-center justify-center r-lg rounded border border-border text-text-muted hover:text-text transition-colors"
            aria-label={t("toggleMenu")}
          >
            {mobileOpen ? (
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="s-1">
                <path d="M4 4l8 8M12 4l-8 8" />
              </svg>
            ) : (
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="s-1">
                <path d="M2 4h12M2 8h12M2 12h12" />
              </svg>
            )}
          </button>
        </div>

        {/* Mobile menu */}
        {mobileOpen && (
          <nav className="md:hidden border-t border-border bg-surface p-h-15 p-v-1">
            {navItems.map((item) =>
              item.external ? (
                <a
                  key={item.key}
                  href={`${item.href}/${locale}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="block p-v-05 text-sm text-text-muted hover:text-text transition-colors"
                  onClick={() => setMobileOpen(false)}
                >
                  {t(item.key)}
                </a>
              ) : (
                <Link
                  key={item.key}
                  href={item.href}
                  className="block p-v-05 text-sm text-text-muted hover:text-text transition-colors"
                  onClick={() => setMobileOpen(false)}
                >
                  {t(item.key)}
                </Link>
              ),
            )}

            {/* Mobile language selector trigger */}
            <div className="border-t border-border m-t-075 p-v-075">
              <button
                onClick={() => setLangModalOpen(true)}
                className="flex items-center g-025 r-pill rounded p-v-05 p-h-075 border border-border bg-surface/50 text-sm text-text-muted transition-colors hover:text-text hover:border-accent/40"
                aria-label="Select language"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="icon-sm">
                  <circle cx="12" cy="12" r="10" />
                  <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10A15.3 15.3 0 0 1 12 2z" />
                </svg>
                {localeLabels[locale]}
              </button>
            </div>
          </nav>
        )}
      </header>

      {/* Language selector modal */}
      {langModalOpen && (
        <div
          className="overlay-backdrop flex items-center justify-center"
          onClick={closeLangModal}
          role="dialog"
          aria-modal="true"
          aria-label="Select language"
        >
          <div className="absolute inset-0 bg-bg/60 backdrop-blur-sm" />
          <div
            className="overlay-panel relative w-full max-w-xs r-2xl rounded-p-1 border border-border bg-surface shadow-lg"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between m-b-075">
              <span className="text-sm font-medium text-text">Select language</span>
              <button
                onClick={closeLangModal}
                className="flex s-15 items-center justify-center r-pill rounded text-text-muted hover:text-text transition-colors"
                aria-label="Close"
              >
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className="icon-sm">
                  <path d="M4 4l8 8M12 4l-8 8" />
                </svg>
              </button>
            </div>
            <div className="flex flex-col g-025">
              {locales.map((loc) => (
                <button
                  key={loc}
                  onClick={() => switchLocale(loc)}
                  className={`flex items-center justify-between r-lg rounded p-v-05 p-h-075 text-sm transition-colors ${
                    locale === loc
                      ? "bg-accent/10 text-accent"
                      : "text-text-muted hover:bg-surface-alt hover:text-text"
                  }`}
                >
                  <span>{localeFullNames[loc]}</span>
                  <span className="text-xs opacity-60">{localeLabels[loc]}</span>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
