export const siteConfig = {
  name: "ADI",
  description: "Artificial Developer Intelligence — autonomous AI infrastructure for software engineering",
  url: process.env.NEXT_PUBLIC_SITE_URL || "https://adi.the-ihor.com",
  github: "https://github.com/adi-family/adi-cli",
  twitter: "@mgorunuch",
  locale: "en_US",
  author: {
    name: "ADI Team",
    url: "https://adi.the-ihor.com",
  },
} as const;
