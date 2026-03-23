import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AnimatePresence } from "framer-motion";
import { Filmstrip } from "./components/Filmstrip";
import { PasteStackView } from "./components/Filmstrip/PasteStackView";
import { Search, type SearchFilters } from "./components/Search";
import { useSearch } from "./hooks/useSearch";
import { usePinboards } from "./hooks/usePinboards";
import { useTheme } from "./hooks/useTheme";
import { PinboardView, PinboardPicker, CreatePinboardDialog } from "./components/Pinboard";
import { useSnippets } from "./hooks/useSnippets";
import { SnippetView } from "./components/Snippet";
import { CardPreview } from "./components/Card/CardPreview";
import { ClipEditor } from "./components/Card/ClipEditor";
import { Settings } from "./components/Settings";

export interface ClipData {
  id: string;
  content_type: string;
  text_content: string | null;
  html_content: string | null;
  image_path: string | null;
  source_app: string | null;
  source_app_icon: string | null;
  content_hash: string;
  content_size: number;
  metadata: string | null;
  pinboard_id: string | null;
  is_favorite: boolean;
  created_at: string;
  accessed_at: string | null;
  access_count: number;
}

type TabView = "history" | "pinboards" | "snippets" | "settings";

function App() {
  const { resolvedTheme: _resolvedTheme } = useTheme();
  const [clips, setClips] = useState<ClipData[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [multiSelectedIds, setMultiSelectedIds] = useState<Set<string>>(new Set());
  const [activeTab, setActiveTab] = useState<TabView>("history");
  const [loading, setLoading] = useState(true);
  const [hasMore, setHasMore] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const PAGE_SIZE = 50;
  const containerRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const { results, isSearching, loading: searchLoading, search, clearSearch } = useSearch();

  // Pinboard state
  const {
    pinboards,
    reload: reloadPinboards,
    createPinboard,
    updatePinboard,
    deletePinboard,
    addClipToPinboard,
  } = usePinboards();
  const {
    snippets,
    groups: snippetGroups,
    loading: snippetsLoading,
    reload: reloadSnippets,
    createSnippet,
    updateSnippet,
    deleteSnippet,
    createGroup: createSnippetGroup,
    deleteGroup: deleteSnippetGroup,
  } = useSnippets();
  const [showPinboardPicker, setShowPinboardPicker] = useState(false);
  const [showCreatePinboard, setShowCreatePinboard] = useState(false);
  const [showPasteStack, setShowPasteStack] = useState(false);
  const [pasteStackActive, setPasteStackActive] = useState(false);
  const [pasteStackCount, setPasteStackCount] = useState(0);
  const [showPreview, setShowPreview] = useState(false);
  const [showEditor, setShowEditor] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; clipId: string } | null>(null);

  // The clips to display: search results when searching, all clips otherwise
  const displayClips = useMemo(
    () => (isSearching ? results : clips),
    [isSearching, results, clips],
  );
  const displayLoading = useMemo(
    () => (isSearching ? searchLoading : loading),
    [isSearching, searchLoading, loading],
  );

  const loadClips = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<ClipData[]>("get_clips", {
        offset: 0,
        limit: PAGE_SIZE,
      });
      setClips(result);
      setHasMore(result.length === PAGE_SIZE);
      setSelectedIndex(0);
    } catch (err) {
      console.error("Failed to load clips:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadMoreClips = useCallback(async () => {
    if (loadingMore || !hasMore) return;
    try {
      setLoadingMore(true);
      const result = await invoke<ClipData[]>("get_clips", {
        offset: clips.length,
        limit: PAGE_SIZE,
      });
      if (result.length > 0) {
        setClips((prev) => [...prev, ...result]);
      }
      setHasMore(result.length === PAGE_SIZE);
    } catch (err) {
      console.error("Failed to load more clips:", err);
    } finally {
      setLoadingMore(false);
    }
  }, [clips.length, hasMore, loadingMore]);

  useEffect(() => {
    loadClips();
  }, [loadClips]);

  // Listen for new clips from the backend clipboard monitor
  useEffect(() => {
    const unlisten = listen("clip-added", () => {
      loadClips();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadClips]);

  // Auto-focus search bar when window becomes visible
  useEffect(() => {
    const handleFocus = () => {
      setTimeout(() => searchRef.current?.focus(), 50);
    };
    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, []);

  const pasteSelected = useCallback(async () => {
    if (displayClips.length === 0) return;
    const clip = displayClips[selectedIndex];
    if (!clip) return;
    try {
      await invoke("paste_clip", { id: clip.id });
    } catch (err) {
      console.error("Failed to paste:", err);
    }
  }, [displayClips, selectedIndex]);

  const handleCardContextMenu = useCallback((index: number, event: React.MouseEvent) => {
    setSelectedIndex(index);
    setContextMenu({
      x: event.clientX,
      y: event.clientY,
      clipId: displayClips[index]?.id || "",
    });
  }, [displayClips]);

  const copySelected = useCallback(async () => {
    if (displayClips.length === 0) return;
    const clip = displayClips[selectedIndex];
    if (!clip) return;
    try {
      await invoke("copy_to_clipboard", { id: clip.id });
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  }, [displayClips, selectedIndex]);

  const deleteSelected = useCallback(async () => {
    if (displayClips.length === 0) return;
    const clip = displayClips[selectedIndex];
    if (!clip) return;
    try {
      await invoke("delete_clip", { id: clip.id });
      await loadClips();
    } catch (err) {
      console.error("Failed to delete:", err);
    }
  }, [displayClips, selectedIndex, loadClips]);

  const pastePlainSelected = useCallback(async () => {
    if (displayClips.length === 0) return;
    const clip = displayClips[selectedIndex];
    if (!clip) return;
    try {
      await invoke("paste_clip_plain", { id: clip.id });
    } catch (err) {
      console.error("Failed to paste plain:", err);
    }
  }, [displayClips, selectedIndex]);

  const toggleFavoriteSelected = useCallback(async () => {
    if (displayClips.length === 0) return;
    const clip = displayClips[selectedIndex];
    if (!clip) return;
    try {
      await invoke("toggle_favorite", { id: clip.id });
      await loadClips();
    } catch (err) {
      console.error("Failed to toggle favorite:", err);
    }
  }, [displayClips, selectedIndex, loadClips]);

  const handleSearch = useCallback(
    (query: string, filters: SearchFilters) => {
      search(query, filters);
      setSelectedIndex(0);
    },
    [search],
  );

  const handleClearSearch = useCallback(() => {
    clearSearch();
    setSelectedIndex(0);
  }, [clearSearch]);

  const handleCardSelect = useCallback((index: number, event?: React.MouseEvent) => {
    if (event?.ctrlKey || event?.metaKey) {
      // Ctrl+click: toggle multi-selection
      const clipId = displayClips[index]?.id;
      if (clipId) {
        setMultiSelectedIds(prev => {
          const next = new Set(prev);
          if (next.has(clipId)) {
            next.delete(clipId);
          } else {
            next.add(clipId);
          }
          return next;
        });
      }
    } else if (event?.shiftKey && multiSelectedIds.size > 0) {
      // Shift+click: range selection from selectedIndex to clicked index
      const start = Math.min(selectedIndex, index);
      const end = Math.max(selectedIndex, index);
      const newIds = new Set(multiSelectedIds);
      for (let i = start; i <= end; i++) {
        const clip = displayClips[i];
        if (clip) newIds.add(clip.id);
      }
      setMultiSelectedIds(newIds);
    } else {
      // Normal click: clear multi-selection, set single selection
      setMultiSelectedIds(new Set());
      setSelectedIndex(index);
    }
  }, [displayClips, selectedIndex, multiSelectedIds]);

  const handlePinboardSelect = useCallback(async (pinboardId: string) => {
    const clip = displayClips[selectedIndex];
    if (!clip) return;
    await addClipToPinboard(clip.id, pinboardId);
    setShowPinboardPicker(false);
    await loadClips();
  }, [displayClips, selectedIndex, addClipToPinboard, loadClips]);

  const handleCreatePinboardFromPicker = useCallback(async (name: string, color: string) => {
    try {
      // Create the pinboard and get its ID back
      const newPinboard = await invoke<{ id: string }>("create_pinboard", { name, color });
      // Save the selected clip to the new pinboard
      const clip = displayClips[selectedIndex];
      if (clip && newPinboard?.id) {
        await addClipToPinboard(clip.id, newPinboard.id);
        await loadClips();
      }
    } catch (err) {
      console.error("Failed to create pinboard and save clip:", err);
    }
    setShowCreatePinboard(false);
  }, [createPinboard, displayClips, selectedIndex, addClipToPinboard, loadClips]);

  const handlePasteStackStatusChange = useCallback((active: boolean, count: number) => {
    setPasteStackActive(active);
    setPasteStackCount(count);
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Check if search input is focused
      const isSearchFocused = document.activeElement === searchRef.current;

      // Ctrl+E to edit selected clip
      if (e.key === "e" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        const clip = displayClips[selectedIndex];
        if (clip && (clip.content_type === "text" || clip.content_type === "code")) {
          setShowEditor(true);
        }
        return;
      }

      // Ctrl+P to open pinboard picker
      if (e.key === "p" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        if (displayClips.length > 0 && displayClips[selectedIndex]) {
          setShowPinboardPicker(true);
        }
        return;
      }

      // / or Ctrl+F to focus search
      if (
        (e.key === "/" && !isSearchFocused) ||
        (e.key === "f" && (e.ctrlKey || e.metaKey))
      ) {
        e.preventDefault();
        searchRef.current?.focus();
        return;
      }

      // Option+Arrow to switch tabs
      if (e.altKey && (e.key === "ArrowRight" || e.key === "ArrowLeft")) {
        e.preventDefault();
        const tabs: TabView[] = ["history", "pinboards", "snippets", "settings"];
        setActiveTab((prev) => {
          const idx = tabs.indexOf(prev);
          if (e.key === "ArrowRight") {
            return tabs[(idx + 1) % tabs.length];
          } else {
            return tabs[(idx - 1 + tabs.length) % tabs.length];
          }
        });
        setShowPasteStack(false);
        return;
      }

      // When search is focused, allow arrow keys, Enter, and Escape through
      // Block Space, F, Delete etc. so they type into the search bar
      if (isSearchFocused && !["ArrowLeft", "ArrowRight", "Enter", "Escape"].includes(e.key)) return;

      switch (e.key) {
        case " ": // Space — Quick Look preview
          e.preventDefault();
          if (displayClips.length > 0 && displayClips[selectedIndex]) {
            setShowPreview((prev) => !prev);
          }
          break;
        case "Escape":
          e.preventDefault();
          if (multiSelectedIds.size > 0) {
            setMultiSelectedIds(new Set());
          } else if (showPreview) {
            setShowPreview(false);
          } else if (isSearching) {
            handleClearSearch();
          } else {
            // Dismiss the overlay
            dismissOverlay();
          }
          break;
        case "ArrowRight":
          e.preventDefault();
          setSelectedIndex((prev) =>
            Math.min(prev + 1, displayClips.length - 1),
          );
          break;
        case "ArrowLeft":
          e.preventDefault();
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
          break;
        case "Enter":
          e.preventDefault();
          if (multiSelectedIds.size > 0) {
            // Multi-paste: concatenate text from selected clips in display order
            const ids = displayClips
              .filter(c => multiSelectedIds.has(c.id))
              .map(c => c.id);
            if (ids.length > 0) {
              invoke("paste_clips_multi", { ids }).catch(err =>
                console.error("Multi-paste failed:", err)
              );
              setMultiSelectedIds(new Set());
            }
          } else {
            // Copy to clipboard (same as double-click)
            copySelected();
          }
          break;
        case "Delete":
        case "Backspace":
          e.preventDefault();
          deleteSelected();
          break;
        case "f":
          if (!e.ctrlKey && !e.metaKey) {
            e.preventDefault();
            toggleFavoriteSelected();
          }
          break;
        case "Tab":
          e.preventDefault();
          setActiveTab((prev) => {
            const tabs: TabView[] = ["history", "pinboards", "snippets", "settings"];
            const idx = tabs.indexOf(prev);
            return tabs[(idx + 1) % tabs.length];
          });
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [displayClips, selectedIndex, multiSelectedIds, pasteSelected, pastePlainSelected, deleteSelected, toggleFavoriteSelected, showPreview, showEditor, isSearching, handleClearSearch]);

  // Close preview and editor when selection changes
  useEffect(() => {
    setShowPreview(false);
    setShowEditor(false);
  }, [selectedIndex]);

  // Scroll selected card into view
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const selectedCard = container.querySelector(
      `[data-index="${selectedIndex}"]`,
    );
    if (selectedCard) {
      selectedCard.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
    }
  }, [selectedIndex]);

  const dismissOverlay = useCallback(() => {
    const win = (window as any).__TAURI__?.window?.getCurrentWindow?.();
    if (win) {
      win.hide();
    } else {
      // Fallback: use invoke to call a hide command
      invoke("hide_overlay").catch(() => {});
    }
  }, []);

  return (
    <div className="flex h-screen flex-col text-text-primary select-none">
      {/* Backdrop — click to dismiss */}
      <div
        className="flex-1 cursor-pointer"
        onClick={dismissOverlay}
        onMouseDown={(e) => {
          e.preventDefault();
          dismissOverlay();
        }}
      />
      {/* Filmstrip panel at bottom */}
      <div className="flex flex-col bg-surface-bg border-t border-border-default shadow-2xl">
      {/* Tab bar */}
      <div className="flex items-center gap-1 border-b border-border-default px-4 py-2" role="tablist" aria-label="Content views">
        {(["history", "pinboards", "snippets"] as TabView[]).map((tab) => (
          <button
            key={tab}
            role="tab"
            aria-selected={activeTab === tab && !showPasteStack}
            onClick={() => {
              setActiveTab(tab);
              setShowPasteStack(false);
            }}
            className={`font-heading rounded-md px-4 py-1.5 text-base font-semibold capitalize tracking-wide transition-colors ${
              activeTab === tab && !showPasteStack
                ? "bg-surface-hover text-text-primary"
                : "text-text-muted hover:text-text-secondary"
            }`}
          >
            {tab}
          </button>
        ))}
        <div className="flex-1" />
        <button
          onClick={() => setShowPasteStack((prev) => !prev)}
          className={`font-heading relative rounded-md px-3 py-1 text-sm font-semibold tracking-wide transition-colors ${
            showPasteStack
              ? "bg-surface-hover text-text-primary"
              : "text-text-muted hover:text-text-secondary"
          }`}
        >
          Stack
          {pasteStackActive && (
            <span className="absolute -top-1 -right-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-accent px-1 text-[10px] font-bold text-white">
              {pasteStackCount}
            </span>
          )}
        </button>
        <button
          onClick={() => {
            setActiveTab("settings");
            setShowPasteStack(false);
          }}
          className={`rounded-md p-1.5 transition-colors ${
            activeTab === "settings" && !showPasteStack
              ? "bg-surface-hover text-text-primary"
              : "text-text-muted hover:text-text-secondary"
          }`}
          title="Settings"
        >
          <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
        </button>
        <span className="text-xs text-text-muted" aria-live="polite">
          {multiSelectedIds.size > 0
            ? `${multiSelectedIds.size} selected`
            : displayClips.length > 0
              ? `${displayClips.length} item${displayClips.length !== 1 ? "s" : ""}`
              : ""}
        </span>
      </div>

      {/* Search bar */}
      {activeTab === "history" && (
        <Search
          onSearch={handleSearch}
          onClear={handleClearSearch}
          searchRef={searchRef}
        />
      )}

      {/* Tab content */}
      <div role="tabpanel" aria-label={`${showPasteStack ? "stack" : activeTab} content`}>
      {showPasteStack ? (
        <PasteStackView onStatusChange={handlePasteStackStatusChange} />
      ) : activeTab === "history" ? (
        <Filmstrip
          clips={displayClips}
          selectedIndex={selectedIndex}
          multiSelectedIds={multiSelectedIds}
          onSelect={handleCardSelect}
          onPaste={copySelected}
          onCardContextMenu={handleCardContextMenu}
          loading={displayLoading}
          containerRef={containerRef}
          onLoadMore={isSearching ? undefined : loadMoreClips}
          hasMore={isSearching ? false : hasMore}
          loadingMore={isSearching ? false : loadingMore}
          onClipCreated={loadClips}
        />
      ) : activeTab === "pinboards" ? (
        <PinboardView
          pinboards={pinboards}
          onReload={reloadPinboards}
          onCreatePinboard={createPinboard}
          onUpdatePinboard={updatePinboard}
          onDeletePinboard={deletePinboard}
        />
      ) : activeTab === "snippets" ? (
        <SnippetView
          snippets={snippets}
          groups={snippetGroups}
          onCreateSnippet={createSnippet}
          onUpdateSnippet={updateSnippet}
          onDeleteSnippet={deleteSnippet}
          onCreateGroup={createSnippetGroup}
          onDeleteGroup={deleteSnippetGroup}
          onReload={reloadSnippets}
          loading={snippetsLoading}
        />
      ) : activeTab === "settings" ? (
        <Settings />
      ) : null}
      </div>

      {/* Pinboard picker modal */}
      {showPinboardPicker && (
        <PinboardPicker
          pinboards={pinboards}
          onSelect={handlePinboardSelect}
          onCreateNew={() => {
            setShowPinboardPicker(false);
            setShowCreatePinboard(true);
          }}
          onClose={() => setShowPinboardPicker(false)}
        />
      )}

      {/* Create pinboard dialog (from picker) */}
      {showCreatePinboard && (
        <CreatePinboardDialog
          title="Create Pinboard"
          onSave={handleCreatePinboardFromPicker}
          onClose={() => setShowCreatePinboard(false)}
        />
      )}

      {/* Quick Look preview */}
      <AnimatePresence>
        {showPreview && displayClips[selectedIndex] && (
          <CardPreview
            clip={displayClips[selectedIndex]}
            onClose={() => setShowPreview(false)}
            onPaste={pasteSelected}
          />
        )}
      </AnimatePresence>

      {/* Clip editor modal */}
      <AnimatePresence>
        {showEditor && displayClips[selectedIndex] && (
          <ClipEditor
            clip={displayClips[selectedIndex]}
            onSave={loadClips}
            onClose={() => setShowEditor(false)}
          />
        )}
      </AnimatePresence>

      {/* Right-click context menu */}
      {contextMenu && (
        <div
          className="fixed inset-0 z-50"
          onClick={() => setContextMenu(null)}
        >
          <div
            className="absolute rounded-lg border border-border-default bg-surface-primary py-1 shadow-xl"
            style={{ left: contextMenu.x, top: contextMenu.y }}
            onClick={(e) => e.stopPropagation()}
          >
            <button
              onClick={async () => {
                await invoke("copy_to_clipboard", { id: contextMenu.clipId });
                setContextMenu(null);
              }}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-xs text-text-secondary hover:bg-surface-hover"
            >
              Copy to clipboard
            </button>
            <button
              onClick={() => {
                setShowPinboardPicker(true);
                setContextMenu(null);
              }}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-xs text-text-secondary hover:bg-surface-hover"
            >
              Save to pinboard...
            </button>
            <button
              onClick={async () => {
                await invoke("toggle_favorite", { id: contextMenu.clipId });
                await loadClips();
                setContextMenu(null);
              }}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-xs text-text-secondary hover:bg-surface-hover"
            >
              Toggle favorite
            </button>
            <div className="my-1 border-t border-border-subtle" />
            <button
              onClick={async () => {
                await invoke("delete_clip", { id: contextMenu.clipId });
                await loadClips();
                setContextMenu(null);
              }}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-xs text-red-400 hover:bg-surface-hover"
            >
              Delete
            </button>
          </div>
        </div>
      )}

      {/* Footer with keyboard hints */}
      <div
        className="flex items-center gap-3 border-t border-border-default px-4 py-2 text-xs text-text-muted font-heading tracking-wide"
        role="toolbar"
        aria-label="Keyboard shortcuts"
      >
        {[
          "←→ Navigate",
          "↵ Paste",
          "⇧↵ Plain",
          "␣ Preview",
          "F Fav",
          "/ Search",
          "⌃E Edit",
          "⌃P Pin",
          "⌫ Remove",
          "⇥ Views",
          "Esc Close",
        ].map((hint, i) => (
          <span key={hint} className="flex items-center gap-2">
            {i > 0 && <span className="text-text-faint">·</span>}
            {hint}
          </span>
        ))}
      </div>
      </div>{/* end filmstrip panel */}
    </div>
  );
}

export default App;
