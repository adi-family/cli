import { notFound } from "next/navigation";
import { setRequestLocale, getTranslations } from "next-intl/server";
import type { Metadata } from "next";
import { siteConfig } from "@/lib/site";
import { getAudienceByPersona, getAllPersonaSlugs } from "@/lib/audience";
import { routing } from "@/i18n/routing";
import { UnderConstruction } from "@/components/feedback/under-construction";

type Props = {
  params: Promise<{ locale: string; persona: string }>;
};

export function generateStaticParams() {
  const personas = getAllPersonaSlugs();
  return routing.locales.flatMap((locale) =>
    personas.map((persona) => ({ locale, persona })),
  );
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { locale, persona } = await params;
  const audience = getAudienceByPersona(persona);
  if (!audience) return {};

  const t = await getTranslations({ locale, namespace: "personas" });
  const title = t(`${persona}.metaTitle`);
  const description = t(`${persona}.metaDescription`);

  return {
    title,
    description,
    openGraph: {
      title,
      description,
      url: `${siteConfig.url}/for/${persona}`,
      type: "website",
    },
  };
}

export default async function PersonaPage({ params }: Props) {
  const { locale, persona } = await params;
  setRequestLocale(locale);

  const audience = getAudienceByPersona(persona);
  if (!audience) notFound();

  const t = await getTranslations({ locale, namespace: "underConstruction" });

  return (
    <UnderConstruction
      heading={t("title")}
      description={t("description")}
      badge={t("badge")}
    />
  );
}
