import { ImageResponse } from "next/og";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

export const dynamic = "force-static";

export const alt = "Nous — The sovereign everything-app";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default async function TwitterImage() {
  const geist = await readFile(
    join(
      process.cwd(),
      "node_modules/next/dist/compiled/@vercel/og/Geist-Regular.ttf",
    ),
  );

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          background: "#000000",
          fontFamily: "Geist",
          position: "relative",
          overflow: "hidden",
        }}
      >
        {/* Radial glow */}
        <div
          style={{
            position: "absolute",
            width: 700,
            height: 700,
            borderRadius: "50%",
            background:
              "radial-gradient(circle, rgba(212,175,55,0.08) 0%, transparent 70%)",
            top: "50%",
            left: "50%",
            transform: "translate(-50%, -50%)",
          }}
        />

        {/* Title */}
        <span
          style={{
            fontSize: 108,
            fontWeight: 200,
            letterSpacing: "-0.05em",
            color: "#ffffff",
            lineHeight: 1,
          }}
        >
          Nous
        </span>
        <span
          style={{
            fontSize: 22,
            fontWeight: 300,
            color: "#737373",
            marginTop: 16,
          }}
        >
          The sovereign everything-app.
        </span>

        {/* Tagline */}
        <span
          style={{
            fontSize: 14,
            color: "#404040",
            marginTop: 32,
            letterSpacing: "0.05em",
          }}
        >
          Identity / Messaging / Governance / Payments / AI — encrypted &
          decentralized
        </span>

        {/* Gold line */}
        <div
          style={{
            position: "absolute",
            bottom: 0,
            left: 0,
            right: 0,
            height: 2,
            background:
              "linear-gradient(90deg, transparent, #d4af37, transparent)",
          }}
        />
      </div>
    ),
    {
      ...size,
      fonts: [
        {
          name: "Geist",
          data: geist,
          style: "normal",
          weight: 400,
        },
      ],
    },
  );
}
