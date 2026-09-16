import "@testing-library/jest-dom/vitest";
import React from "react";
import { vi } from "vitest";

// Mock window.matchMedia (not available in jsdom)
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// jsdom has no layout engine, so scrollIntoView is unimplemented. Components
// call it to keep the selected card on screen; make it a no-op spy.
Element.prototype.scrollIntoView = vi.fn();

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  // Default to a resolved promise: the real invoke always returns one, and
  // components legitimately chain .then/.catch onto it. A bare vi.fn() returns
  // undefined and blows up any component that awaits a command it did not
  // explicitly stub.
  invoke: vi.fn(() => Promise.resolve(null)),
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
}));

// Mock framer-motion to avoid animation issues in tests
vi.mock("framer-motion", () => ({
  motion: new Proxy(
    {},
    {
      get: (_target, prop) => {
        return React.forwardRef(({ children, ...props }: any, ref: any) =>
          React.createElement(prop as string, { ...props, ref }, children),
        );
      },
    },
  ),
  AnimatePresence: ({ children }: any) => children,
}));
