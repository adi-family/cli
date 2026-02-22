import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

export default function NotFound() {
  const t = useTranslations("notFound");

  return (
    <div className="flex min-h-[60vh] flex-col items-center justify-center px-6 text-center">
      <span className="font-mono text-6xl font-bold text-accent/20">{t("code")}</span>
      <h1 className="mt-4 font-heading text-2xl font-semibold text-text">
        {t("title")}
      </h1>
      <p className="mt-2 text-text-muted">
        {t("description")}
      </p>
      <Link
        href="/"
        className="mt-8 inline-flex items-center rounded-full border border-border px-6 py-2.5 text-sm text-text-muted hover:text-text hover:border-accent/40 transition-colors"
      >
        &larr; {t("backHome")}
      </Link>
    </div>
  );
}
