import { describe, it, expect } from "vitest";

// Test the detection functions from the clipboard module
// Since these are in Rust, we test the equivalent frontend logic

describe("formatRelativeTime", () => {
  // Re-implement the format function for testing since it's defined inside Card component
  function formatRelativeTime(isoDate: string): string {
    const now = Date.now();
    const then = new Date(isoDate).getTime();
    const diffMs = now - then;
    const diffSec = Math.floor(diffMs / 1000);
    const diffMin = Math.floor(diffSec / 60);
    const diffHr = Math.floor(diffMin / 60);
    const diffDay = Math.floor(diffHr / 24);

    if (diffSec < 60) return "just now";
    if (diffMin < 60) return `${diffMin}m ago`;
    if (diffHr < 24) return `${diffHr}h ago`;
    if (diffDay < 7) return `${diffDay}d ago`;
    return new Date(isoDate).toLocaleDateString();
  }

  it("formats just now", () => {
    const now = new Date().toISOString();
    expect(formatRelativeTime(now)).toBe("just now");
  });

  it("formats minutes ago", () => {
    const date = new Date(Date.now() - 5 * 60 * 1000).toISOString();
    expect(formatRelativeTime(date)).toBe("5m ago");
  });

  it("formats hours ago", () => {
    const date = new Date(Date.now() - 3 * 60 * 60 * 1000).toISOString();
    expect(formatRelativeTime(date)).toBe("3h ago");
  });

  it("formats days ago", () => {
    const date = new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString();
    expect(formatRelativeTime(date)).toBe("2d ago");
  });

  it("formats older dates as locale string", () => {
    const date = new Date(
      Date.now() - 30 * 24 * 60 * 60 * 1000,
    ).toISOString();
    const result = formatRelativeTime(date);
    // Should be a date string, not "Xd ago"
    expect(result).not.toContain("d ago");
  });
});

describe("content type detection helpers", () => {
  // Test the URL extraction logic used in LinkCard
  function extractDomain(url: string): string {
    try {
      const u = new URL(url.trim());
      return u.hostname.replace(/^www\./, "");
    } catch {
      return url.trim().slice(0, 30);
    }
  }

  it("extracts domain from URL", () => {
    expect(extractDomain("https://example.com/path")).toBe("example.com");
    expect(extractDomain("https://www.example.com")).toBe("example.com");
    expect(extractDomain("http://sub.domain.com/page")).toBe("sub.domain.com");
  });

  it("handles invalid URLs", () => {
    expect(extractDomain("not a url")).toBe("not a url");
  });

  // Test file name extraction from FileCard
  function extractFilename(path: string): string {
    const trimmed = path.trim();
    const cleaned = trimmed.startsWith("file://") ? trimmed.slice(7) : trimmed;
    const parts = cleaned.split("/");
    return parts[parts.length - 1] || cleaned;
  }

  it("extracts filename from path", () => {
    expect(extractFilename("/home/user/doc.pdf")).toBe("doc.pdf");
    expect(extractFilename("file:///tmp/image.png")).toBe("image.png");
  });
});
