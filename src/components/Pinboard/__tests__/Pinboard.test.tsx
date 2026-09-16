import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { PinboardView } from "../index";
import { mockClips } from "../../../test/fixtures";
import type { PinboardData } from "../../../hooks/usePinboards";

const mockInvoke = vi.mocked(invoke);

const pinboards: PinboardData[] = [
  { id: "pb-1", name: "Work", color: "#ff0000", icon: null, position: 0, created_at: new Date().toISOString() },
  { id: "pb-2", name: "Links", color: "#00ff00", icon: null, position: 1, created_at: new Date().toISOString() },
];

// Only text-bearing clips — enough to distinguish which one was confirmed.
const pinnedClips = mockClips.slice(0, 3).map((c) => ({ ...c, pinboard_id: "pb-1" }));

function renderView(onConfirmClip = vi.fn()) {
  const utils = render(
    <PinboardView
      pinboards={pinboards}
      onReload={() => {}}
      onCreatePinboard={() => {}}
      onUpdatePinboard={() => {}}
      onDeletePinboard={() => {}}
      onConfirmClip={onConfirmClip}
    />,
  );
  return { ...utils, onConfirmClip };
}

/** Open the "Work" pinboard and wait for its clips to render. */
async function openWorkPinboard(onConfirmClip = vi.fn()) {
  const utils = renderView(onConfirmClip);
  fireEvent.click(screen.getByText("Work"));
  await waitFor(() => expect(screen.getByText(/Hello, world!/)).toBeInTheDocument());
  // Flush passive effects before any test fires a key.
  //
  // PinboardView registers its window keydown listener in an effect gated on
  // `clips.length === 0`, so the listener only exists once clips have loaded.
  // waitFor resolves as soon as the clip text is in the DOM, which can be
  // before that effect has run — firing into that gap drops the key and the
  // test then times out waiting for a selection change that can never happen.
  // Only reachable in tests; a user cannot press a key that fast.
  await act(async () => {});
  return utils;
}

function cards(container: HTMLElement) {
  return Array.from(container.querySelectorAll("[data-index]"));
}

function selectedCard(container: HTMLElement) {
  return container.querySelector('[data-index][aria-selected="true"]');
}

describe("PinboardView", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(pinnedClips);
  });

  it("lists pinboards before one is opened", () => {
    renderView();
    expect(screen.getByText("Work")).toBeInTheDocument();
    expect(screen.getByText("Links")).toBeInTheDocument();
  });

  it("loads the clips scoped to the opened pinboard", async () => {
    await openWorkPinboard();
    expect(mockInvoke).toHaveBeenCalledWith("get_clips", {
      offset: 0,
      limit: 100,
      pinboardId: "pb-1",
    });
  });

  it("highlights the first clip once a pinboard is opened", async () => {
    const { container } = await openWorkPinboard();
    expect(cards(container)).toHaveLength(pinnedClips.length);
    expect(selectedCard(container)).toBe(cards(container)[0]);
  });

  it("moves the highlight with left/right arrows", async () => {
    const { container } = await openWorkPinboard();

    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => expect(selectedCard(container)).toBe(cards(container)[1]));

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    await waitFor(() => expect(selectedCard(container)).toBe(cards(container)[0]));
  });

  it("clamps the highlight at both ends of the strip", async () => {
    const { container } = await openWorkPinboard();

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    await waitFor(() => expect(selectedCard(container)).toBe(cards(container)[0]));

    for (let i = 0; i < pinnedClips.length + 2; i++) {
      fireEvent.keyDown(window, { key: "ArrowRight" });
    }
    await waitFor(() =>
      expect(selectedCard(container)).toBe(cards(container)[pinnedClips.length - 1]),
    );
  });

  it("clicking a clip highlights it", async () => {
    const { container } = await openWorkPinboard();
    fireEvent.click(cards(container)[2]);
    await waitFor(() => expect(selectedCard(container)).toBe(cards(container)[2]));
  });

  it("Enter confirms the highlighted pinboard clip, not a history clip", async () => {
    const onConfirmClip = vi.fn();
    await openWorkPinboard(onConfirmClip);

    fireEvent.keyDown(window, { key: "ArrowRight" });
    fireEvent.keyDown(window, { key: "Enter" });

    await waitFor(() => expect(onConfirmClip).toHaveBeenCalledTimes(1));
    expect(onConfirmClip).toHaveBeenCalledWith(pinnedClips[1].id);
  });

  it("double-click confirms that clip even when another is highlighted", async () => {
    const onConfirmClip = vi.fn();
    const { container } = await openWorkPinboard(onConfirmClip);

    fireEvent.doubleClick(cards(container)[2]);

    await waitFor(() => expect(onConfirmClip).toHaveBeenCalledWith(pinnedClips[2].id));
    expect(onConfirmClip).toHaveBeenCalledTimes(1);
  });

  it("does not handle arrows or Enter while the pinboard list is showing", async () => {
    const onConfirmClip = vi.fn();
    renderView(onConfirmClip);

    fireEvent.keyDown(window, { key: "ArrowRight" });
    fireEvent.keyDown(window, { key: "Enter" });

    expect(onConfirmClip).not.toHaveBeenCalled();
  });

  it("resets the highlight to the first clip when switching pinboards", async () => {
    const { container } = await openWorkPinboard();

    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => expect(selectedCard(container)).toBe(cards(container)[1]));

    // Back out, then open the other pinboard.
    fireEvent.click(container.querySelector("button")!);
    await waitFor(() => expect(screen.getByText("Links")).toBeInTheDocument());

    mockInvoke.mockResolvedValue(pinnedClips.slice(0, 2));
    fireEvent.click(screen.getByText("Links"));
    await waitFor(() => expect(cards(container)).toHaveLength(2));
    expect(selectedCard(container)).toBe(cards(container)[0]);
  });

  it("renders an empty state and stays inert when a pinboard has no clips", async () => {
    const onConfirmClip = vi.fn();
    mockInvoke.mockResolvedValue([]);
    renderView(onConfirmClip);

    fireEvent.click(screen.getByText("Work"));
    await waitFor(() => expect(screen.getByText("No clips in this pinboard")).toBeInTheDocument());

    fireEvent.keyDown(window, { key: "Enter" });
    expect(onConfirmClip).not.toHaveBeenCalled();
  });

  it("survives a clip-load failure without confirming anything", async () => {
    const onConfirmClip = vi.fn();
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    mockInvoke.mockRejectedValue("boom");
    renderView(onConfirmClip);

    fireEvent.click(screen.getByText("Work"));
    await waitFor(() => expect(screen.getByText("No clips in this pinboard")).toBeInTheDocument());

    fireEvent.keyDown(window, { key: "Enter" });
    expect(onConfirmClip).not.toHaveBeenCalled();
    consoleError.mockRestore();
  });
});
