import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Download",
  description:
    "Download Nous for macOS, Linux, Windows, and Android. One app, every platform. Install the sovereign everything-app in one click.",
  openGraph: {
    title: "Download Nous",
    description:
      "Download Nous for macOS, Linux, Windows, and Android. One click install on every platform.",
  },
};

export default function DownloadLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <>{children}</>;
}
