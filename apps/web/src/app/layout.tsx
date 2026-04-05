import type { Metadata, Viewport } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const viewport: Viewport = {
  viewportFit: "cover",
  themeColor: "#000000",
};

export const metadata: Metadata = {
  title: {
    default: "Nous — The Sovereign Everything-App",
    template: "%s | Nous",
  },
  description:
    "Identity, messaging, governance, payments, AI — unified under one encrypted, decentralized protocol. Own your digital life.",
  keywords: [
    "decentralized",
    "sovereign identity",
    "encrypted messaging",
    "governance",
    "DID",
    "web3",
    "local-first",
    "CRDT",
    "Rust",
    "open source",
  ],
  authors: [{ name: "Teddy Tennant" }],
  creator: "Teddy Tennant",
  metadataBase: new URL("https://nous.sh"),
  openGraph: {
    type: "website",
    locale: "en_US",
    url: "https://nous.sh",
    siteName: "Nous",
    title: "Nous — The Sovereign Everything-App",
    description:
      "Identity, messaging, governance, payments, AI — unified under one encrypted, decentralized protocol.",
  },
  twitter: {
    card: "summary_large_image",
    title: "Nous — The Sovereign Everything-App",
    description:
      "Identity, messaging, governance, payments, AI — unified under one encrypted, decentralized protocol.",
  },
  robots: {
    index: true,
    follow: true,
  },
  icons: {
    icon: [
      { url: "/favicon.ico", sizes: "any" },
      { url: "/icon.svg", type: "image/svg+xml" },
    ],
    apple: "/apple-touch-icon.png",
  },
  manifest: "/manifest.webmanifest",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} dark h-full antialiased`}
    >
      <body className="min-h-full flex flex-col bg-black text-white">
        {children}
      </body>
    </html>
  );
}
