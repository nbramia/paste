import { describe, it, expect, beforeEach, vi } from "vitest";
import { act, render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import App from "../App";
import { mockClips } from "../test/fixtures";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const mockInvoke = vi.mocked(invoke);

const pinnedClips = mockClips.slice(0, 2).map((c) => ({ ...c, pinboard_id: "pb-1" }));

const pinboards = [
  { id: "pb-1", name: "Work", color: "#ff0000", icon: null, position: 0, created_at: new Date().toISOString() },
];

/**
 * Route invoke() by command name. `get_clips` answers with the pinboard's
 * clips when scoped, the full history otherwise — mirroring the backend,
 * where an unscoped query returns pinned clips too.
 */
function routeInvoke(cmd: string, args?: any) {
  switch (cmd) {
    case "get_clips":
      return Promise.resolve(args?.pinboardId ? pinnedClips : mockClips);
    case "list_pinboards":
      return Promise.resolve(pinboards);
    case "list_snippets":
    case "list_snippet_groups":
      return Promise.resolve([]);
    default:
      return Promise.resolve(undefined);
  }
}

function callsTo(cmd: string) {
  return mockInvoke.mock.calls.filter(([name]) => name === cmd);
}

/**
 * Flush React's pending passive effects.
 *
 * These tests drive the app through a window-level keydown listener that App
 * registers in a `useEffect` keyed on `displayClips`. React commits DOM
 * mutations before it runs passive effects, so waiting on rendered content
 * proves the clips are on screen but NOT that the listener has been
 * re-registered with them — the listener can still be closed over the initial
 * empty array. That gap made these tests flaky (~1 run in 6). Awaiting an
 * empty act() drains the effect queue so the handler is current before we
 * dispatch a key.
 */
async function flushEffects() {
  await act(async () => {});
}

async function renderApp() {
  const utils = render(<App />);
  await waitFor(() =>
    expect(screen.getByText(`${mockClips.length} items`)).toBeInTheDocument(),
  );
  await flushEffects();
  return utils;
}

async function openPinboardsTab() {
  fireEvent.click(screen.getByRole("tab", { name: "pinboards" }));
  await waitFor(() => expect(screen.getByText("Work")).toBeInTheDocument());
  await flushEffects();
}

describe("App keyboard routing", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockImplementation(routeInvoke as any);
  });

  describe("history tab", () => {
    it("Enter pastes the highlighted clip", async () => {
      await renderApp();

      fireEvent.keyDown(window, { key: "Enter" });

      await waitFor(() =>
        expect(mockInvoke).toHaveBeenCalledWith("paste_clip", { id: mockClips[0].id }),
      );
      // paste_clip hides the overlay and injects Ctrl+V on the Rust side, so
      // the frontend neither copies nor dismisses.
      expect(callsTo("copy_to_clipboard")).toHaveLength(0);
      expect(callsTo("hide_overlay")).toHaveLength(0);
    });

    it("Enter pastes the clip the arrows moved to", async () => {
      await renderApp();

      fireEvent.keyDown(window, { key: "ArrowRight" });
      fireEvent.keyDown(window, { key: "Enter" });

      await waitFor(() =>
        expect(mockInvoke).toHaveBeenCalledWith("paste_clip", { id: mockClips[1].id }),
      );
    });

    it("falls back to the clipboard when injection fails", async () => {
      const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
      mockInvoke.mockImplementation(((cmd: string, args: any) =>
        cmd === "paste_clip" ? Promise.reject("no injector") : routeInvoke(cmd, args)) as any);

      await renderApp();
      fireEvent.keyDown(window, { key: "Enter" });

      // The action must not be lost: the clip still reaches the clipboard and
      // the overlay gets out of the way so the user can paste manually.
      await waitFor(() =>
        expect(mockInvoke).toHaveBeenCalledWith("copy_to_clipboard", { id: mockClips[0].id }),
      );
      await waitFor(() => expect(callsTo("hide_overlay")).toHaveLength(1));
      consoleError.mockRestore();
    });
  });

  describe("pinboards tab", () => {
    it("Enter does not touch the history list", async () => {
      await renderApp();
      await openPinboardsTab();

      // Still on the pinboard *list* — nothing is selectable yet.
      fireEvent.keyDown(window, { key: "Enter" });
      fireEvent.keyDown(window, { key: "ArrowRight" });

      await waitFor(() => expect(screen.getByText("Work")).toBeInTheDocument());
      expect(callsTo("paste_clip")).toHaveLength(0);
      expect(callsTo("copy_to_clipboard")).toHaveLength(0);
      expect(callsTo("hide_overlay")).toHaveLength(0);
    });

    it("Enter inside a pinboard pastes that pinboard's clip", async () => {
      await renderApp();
      await openPinboardsTab();

      fireEvent.click(screen.getByText("Work"));
      expect(mockInvoke).toHaveBeenCalledWith("get_clips", {
        offset: 0,
        limit: 100,
        pinboardId: "pb-1",
      });
      // Wait for the strip to actually render — the invoke above fires before
      // the clips are in the DOM, and the key handler needs them there.
      await waitFor(() => expect(screen.getByText(/Hello, world!/)).toBeInTheDocument());
      await flushEffects();

      fireEvent.keyDown(window, { key: "ArrowRight" });
      fireEvent.keyDown(window, { key: "Enter" });

      await waitFor(() => expect(callsTo("paste_clip")).toHaveLength(1));
      expect(mockInvoke).toHaveBeenCalledWith("paste_clip", { id: pinnedClips[1].id });
    });

    it("Delete does not delete a history clip", async () => {
      await renderApp();
      await openPinboardsTab();

      fireEvent.keyDown(window, { key: "Delete" });
      fireEvent.keyDown(window, { key: "Backspace" });

      expect(callsTo("delete_clip")).toHaveLength(0);
    });

    it("f does not favorite a history clip", async () => {
      await renderApp();
      await openPinboardsTab();

      fireEvent.keyDown(window, { key: "f" });

      expect(callsTo("toggle_favorite")).toHaveLength(0);
    });

    it("Ctrl+P does not open the pinboard picker for a history clip", async () => {
      await renderApp();
      await openPinboardsTab();

      fireEvent.keyDown(window, { key: "p", ctrlKey: true });

      expect(screen.queryByText("Save to Pinboard")).not.toBeInTheDocument();
    });
  });

  describe("global keys still work off the history tab", () => {
    it("Tab cycles tabs from the pinboards tab", async () => {
      await renderApp();
      await openPinboardsTab();

      fireEvent.keyDown(window, { key: "Tab" });

      await waitFor(() =>
        expect(screen.getByRole("tab", { name: "snippets" })).toHaveAttribute(
          "aria-selected",
          "true",
        ),
      );
    });

    it("Alt+ArrowRight cycles tabs from the pinboards tab", async () => {
      await renderApp();
      await openPinboardsTab();

      fireEvent.keyDown(window, { key: "ArrowRight", altKey: true });

      await waitFor(() =>
        expect(screen.getByRole("tab", { name: "snippets" })).toHaveAttribute(
          "aria-selected",
          "true",
        ),
      );
    });

    it("Escape dismisses the overlay from the pinboards tab", async () => {
      await renderApp();
      await openPinboardsTab();

      fireEvent.keyDown(window, { key: "Escape" });

      await waitFor(() => expect(callsTo("hide_overlay")).toHaveLength(1));
    });
  });
});
