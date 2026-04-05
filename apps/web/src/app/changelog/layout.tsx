import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Changelog",
  description:
    "What's new in Nous. A detailed log of every feature, fix, and improvement shipped across all platforms.",
  openGraph: {
    title: "Nous Changelog",
    description:
      "What's new in Nous. Every feature, fix, and improvement — shipped fast.",
  },
};

export default function ChangelogLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <>{children}</>;
}
