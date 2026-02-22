import { use } from "react";
import { useTranslations } from "next-intl";
import { setRequestLocale } from "next-intl/server";
import { OrganizationSchema, WebsiteSchema } from "@/components/seo/structured-data";
import { UnderConstruction } from "@/components/feedback/under-construction";

type Props = {
  params: Promise<{locale: string}>;
};

export default function HomePage({params}: Props) {
  const {locale} = use(params);
  setRequestLocale(locale);
  const t = useTranslations("underConstruction");

  return (
    <>
      <OrganizationSchema />
      <WebsiteSchema />

      <UnderConstruction
        heading={t("title")}
        description={t("description")}
        badge={t("badge")}
      />
    </>
  );
}
