import { ImageResponse } from "next/og";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

export const dynamic = "force-static";

export const alt = "Nous — The sovereign everything-app";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default async function OGImage() {
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
        {/* Subtle radial gradient orb */}
        <div
          style={{
            position: "absolute",
            width: 800,
            height: 800,
            borderRadius: "50%",
            background:
              "radial-gradient(circle, rgba(212,175,55,0.08) 0%, transparent 70%)",
            top: "50%",
            left: "50%",
            transform: "translate(-50%, -50%)",
          }}
        />

        {/* Grid lines for depth */}
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            flexDirection: "column",
            justifyContent: "space-between",
            padding: "60px 80px",
            opacity: 0.04,
          }}
        >
          {Array.from({ length: 8 }).map((_, i) => (
            <div
              key={i}
              style={{
                width: "100%",
                height: 1,
                background: "white",
              }}
            />
          ))}
        </div>

        {/* Top bar — version + label */}
        <div
          style={{
            position: "absolute",
            top: 48,
            left: 80,
            right: 80,
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
            }}
          >
            <span
              style={{
                fontSize: 11,
                letterSpacing: "0.2em",
                color: "#525252",
                textTransform: "uppercase",
              }}
            >
              v0.1.0
            </span>
            <span
              style={{
                fontSize: 11,
                letterSpacing: "0.15em",
                color: "#d4af37",
                textTransform: "uppercase",
                border: "1px solid rgba(212,175,55,0.3)",
                padding: "3px 10px",
                borderRadius: 3,
              }}
            >
              Private Alpha
            </span>
          </div>
          <span
            style={{
              fontSize: 11,
              letterSpacing: "0.15em",
              color: "#404040",
              textTransform: "uppercase",
            }}
          >
            github.com/teddytennant/nous
          </span>
        </div>

        {/* Main content */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 0,
            position: "relative",
          }}
        >
          <span
            style={{
              fontSize: 120,
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
              fontSize: 24,
              fontWeight: 300,
              color: "#737373",
              marginTop: 16,
              letterSpacing: "-0.01em",
            }}
          >
            The sovereign everything-app.
          </span>
        </div>

        {/* Bottom feature pills */}
        <div
          style={{
            position: "absolute",
            bottom: 48,
            left: 80,
            right: 80,
            display: "flex",
            justifyContent: "center",
            gap: 16,
          }}
        >
          {[
            "Identity",
            "Messaging",
            "Governance",
            "Payments",
            "Social",
            "Storage",
            "AI",
            "Browser",
          ].map((feature) => (
            <span
              key={feature}
              style={{
                fontSize: 11,
                letterSpacing: "0.1em",
                color: "#525252",
                textTransform: "uppercase",
                border: "1px solid rgba(255,255,255,0.06)",
                padding: "5px 14px",
                borderRadius: 3,
              }}
            >
              {feature}
            </span>
          ))}
        </div>

        {/* Gold accent line */}
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
