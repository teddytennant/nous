import type { MetadataRoute } from "next";

export const dynamic = "force-static";

export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      {
        userAgent: "*",
        allow: "/",
        disallow: ["/api/", "/dashboard", "/social", "/messages", "/wallet", "/governance", "/marketplace", "/ai", "/files", "/network", "/identity", "/settings"],
      },
    ],
    sitemap: "https://nous.sh/sitemap.xml",
  };
}
