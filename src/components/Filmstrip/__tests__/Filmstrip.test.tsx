import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Filmstrip } from "../index";
import { mockClips } from "../../../test/fixtures";
import { createRef } from "react";

describe("Filmstrip", () => {
  const containerRef = createRef<HTMLDivElement>();

  it("renders loading state", () => {
    render(
      <Filmstrip
        clips={[]}
        selectedIndex={0}
        multiSelectedIds={new Set()}
        onSelect={() => {}}
        onPaste={() => {}}
        loading={true}
        containerRef={containerRef}
      />,
    );
    expect(screen.getByText("Loading...")).toBeInTheDocument();
  });

  it("renders empty state", () => {
    render(
      <Filmstrip
        clips={[]}
        selectedIndex={0}
        multiSelectedIds={new Set()}
        onSelect={() => {}}
        onPaste={() => {}}
        loading={false}
        containerRef={containerRef}
      />,
    );
    expect(screen.getByText("No clipboard history yet")).toBeInTheDocument();
  });

  it("renders clips as cards", () => {
    render(
      <Filmstrip
        clips={mockClips}
        selectedIndex={0}
        multiSelectedIds={new Set()}
        onSelect={() => {}}
        onPaste={() => {}}
        loading={false}
        containerRef={containerRef}
      />,
    );
    expect(screen.getByText(/Hello, world!/)).toBeInTheDocument();
    expect(screen.getByText("example.com")).toBeInTheDocument();
  });

  it("renders the correct number of clips", () => {
    const { container } = render(
      <Filmstrip
        clips={mockClips}
        selectedIndex={0}
        multiSelectedIds={new Set()}
        onSelect={() => {}}
        onPaste={() => {}}
        loading={false}
        containerRef={containerRef}
      />,
    );
    const cards = container.querySelectorAll("[data-index]");
    expect(cards.length).toBe(mockClips.length);
  });
});
