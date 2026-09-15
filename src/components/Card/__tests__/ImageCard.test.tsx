import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { ImageCard } from "../ImageCard";

const mockInvoke = vi.mocked(invoke);

const META = '{"format":"png","size_bytes":102400,"width":1920,"height":1080}';

describe("ImageCard", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    // mockReset drops the shared default from setup.ts.
    mockInvoke.mockResolvedValue(null);
  });

  it("renders the thumbnail returned for the clip", async () => {
    mockInvoke.mockResolvedValue("data:image/webp;base64,AAAA");

    render(
      <ImageCard clipId="clip-4" imagePath="/img/clip-4.png" metadata={META} />,
    );

    const img = await screen.findByAltText("Clipboard image preview");
    expect(img).toHaveAttribute("src", "data:image/webp;base64,AAAA");
    expect(mockInvoke).toHaveBeenCalledWith("get_clip_thumbnail", {
      id: "clip-4",
    });
  });

  it("falls back to the placeholder when no thumbnail exists", async () => {
    mockInvoke.mockResolvedValue(null);

    render(
      <ImageCard clipId="clip-4" imagePath="/img/clip-4.png" metadata={META} />,
    );

    await waitFor(() => expect(mockInvoke).toHaveBeenCalled());
    expect(screen.getByText("Image")).toBeInTheDocument();
    expect(
      screen.queryByAltText("Clipboard image preview"),
    ).not.toBeInTheDocument();
  });

  it("falls back to the placeholder when the command fails", async () => {
    mockInvoke.mockRejectedValue(new Error("clip not found"));

    render(
      <ImageCard clipId="clip-4" imagePath="/img/clip-4.png" metadata={META} />,
    );

    await waitFor(() => expect(mockInvoke).toHaveBeenCalled());
    expect(screen.getByText("Image")).toBeInTheDocument();
  });

  it("shows dimensions and size from metadata", () => {
    mockInvoke.mockResolvedValue(null);

    render(
      <ImageCard clipId="clip-4" imagePath="/img/clip-4.png" metadata={META} />,
    );

    expect(screen.getByText("1920×1080 · 100 KB")).toBeInTheDocument();
  });

  it("does not ask for a thumbnail when the clip has no image", () => {
    render(<ImageCard clipId="clip-4" imagePath={null} metadata={null} />);
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("survives malformed metadata", () => {
    mockInvoke.mockResolvedValue(null);
    render(
      <ImageCard clipId="clip-4" imagePath="/img/x.png" metadata={"{oops"} />,
    );
    expect(screen.getByText("Image")).toBeInTheDocument();
  });
});
