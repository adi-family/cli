import { ImageResponse } from "next/og";
import type { NextRequest } from "next/server";

export const runtime = "edge";

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const title = searchParams.get("title") || "ADI Blog";
  const description = searchParams.get("description") || "AI-Powered Developer Intelligence";
  const tags = searchParams.get("tags")?.split(",").filter(Boolean) || [];
  const author = searchParams.get("author") || "";
  const readingTime = searchParams.get("readingTime") || "";

  return new ImageResponse(
    (
      <div
        style={{
          height: "100%",
          width: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: "80px",
          background: "#0a0a0a",
          fontFamily: "Inter, system-ui, sans-serif",
          position: "relative",
        }}
      >
        {/* Background glow */}
        <div
          style={{
            position: "absolute",
            top: "-100px",
            right: "-100px",
            width: "600px",
            height: "600px",
            borderRadius: "50%",
            background: "radial-gradient(circle, rgba(135,95,215,0.15) 0%, transparent 70%)",
          }}
        />

        <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
          {/* Tags */}
          {tags.length > 0 && (
            <div style={{ display: "flex", gap: "8px" }}>
              {tags.slice(0, 4).map((tag) => (
                <div
                  key={tag}
                  style={{
                    padding: "4px 12px",
                    borderRadius: "6px",
                    background: "rgba(135,95,215,0.12)",
                    color: "#875fd7",
                    fontSize: "14px",
                    textTransform: "uppercase",
                    letterSpacing: "0.1em",
                  }}
                >
                  {tag.trim()}
                </div>
              ))}
            </div>
          )}

          {/* Title */}
          <div
            style={{
              fontSize: title.length > 40 ? 52 : 64,
              fontWeight: 700,
              color: "#e0e0e0",
              lineHeight: 1.1,
              letterSpacing: "-0.04em",
              maxWidth: "900px",
            }}
          >
            {title}
          </div>

          {/* Description */}
          {description && (
            <div
              style={{
                fontSize: 22,
                color: "#a0a0a0",
                lineHeight: 1.4,
                maxWidth: "750px",
              }}
            >
              {description.length > 120 ? `${description.slice(0, 120)}...` : description}
            </div>
          )}
        </div>

        {/* Footer */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "16px" }}>
            {/* ADI logo */}
            <div
              style={{
                width: "32px",
                height: "32px",
                border: "1.5px solid rgba(255,255,255,0.07)",
                borderRadius: "8px",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <div
                style={{
                  width: "16px",
                  height: "16px",
                  border: "1.5px solid #875fd7",
                  borderRadius: "3px",
                }}
              />
            </div>
            <span style={{ color: "#a0a0a0", fontSize: "18px" }}>ADI Blog</span>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "16px", color: "#707070", fontSize: "16px" }}>
            {author && <span>{author}</span>}
            {readingTime && (
              <>
                <span style={{ color: "#505050" }}>&middot;</span>
                <span>{readingTime} min read</span>
              </>
            )}
          </div>
        </div>
      </div>
    ),
    {
      width: 1200,
      height: 630,
    },
  );
}
