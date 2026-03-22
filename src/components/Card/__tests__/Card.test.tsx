import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Card } from "../index";
import { mockClips } from "../../../test/fixtures";

const defaultProps = {
  index: 0,
  isSelected: false,
  isMultiSelected: false,
  onSelect: () => {},
  onPaste: () => {},
};

describe("Card", () => {
  it("renders text clip with content preview", () => {
    render(<Card clip={mockClips[0]} {...defaultProps} />);
    expect(screen.getByText(/Hello, world!/)).toBeInTheDocument();
  });

  it("renders code clip with language badge", () => {
    render(<Card clip={mockClips[1]} {...defaultProps} />);
    expect(screen.getByText("Rust")).toBeInTheDocument();
    expect(screen.getByText(/fn main/)).toBeInTheDocument();
  });

  it("renders link clip with domain", () => {
    render(<Card clip={mockClips[2]} {...defaultProps} />);
    expect(screen.getByText("example.com")).toBeInTheDocument();
  });

  it("renders image clip with placeholder", () => {
    render(<Card clip={mockClips[3]} {...defaultProps} />);
    expect(screen.getByText("Image")).toBeInTheDocument();
  });

  it("renders file clip with filename", () => {
    render(<Card clip={mockClips[4]} {...defaultProps} />);
    expect(screen.getByText("report.pdf")).toBeInTheDocument();
  });

  it("shows source app in footer", () => {
    render(<Card clip={mockClips[0]} {...defaultProps} />);
    expect(screen.getByText("firefox")).toBeInTheDocument();
  });

  it("shows relative timestamp", () => {
    render(<Card clip={mockClips[0]} {...defaultProps} />);
    expect(screen.getByText("2m ago")).toBeInTheDocument();
  });

  it("applies selected styling", () => {
    const { container } = render(
      <Card clip={mockClips[0]} {...defaultProps} isSelected={true} />,
    );
    const card = container.firstChild as HTMLElement;
    expect(card.className).toContain("border-blue-500");
  });

  it("shows Unknown for missing source app", () => {
    const clip = { ...mockClips[0], source_app: null };
    render(<Card clip={clip} {...defaultProps} />);
    expect(screen.getByText("Unknown")).toBeInTheDocument();
  });

  it("renders content type color indicator", () => {
    const { container } = render(<Card clip={mockClips[0]} {...defaultProps} />);
    const indicator = container.querySelector(".bg-blue-500");
    expect(indicator).toBeInTheDocument();
  });
});
