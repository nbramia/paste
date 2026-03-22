import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
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

type TabView = "history" | "pinboards" | "snippets";

function App() {
  const { resolvedTheme: _resolvedTheme } = useTheme();
  const [clips, setClips] = useState<ClipData[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
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

  const handlePinboardSelect = useCallback(async (pinboardId: string) => {
    const clip = displayClips[selectedIndex];
    if (!clip) return;
    await addClipToPinboard(clip.id, pinboardId);
    setShowPinboardPicker(false);
    await loadClips();
  }, [displayClips, selectedIndex, addClipToPinboard, loadClips]);

  const handleCreatePinboardFromPicker = useCallback(async (name: string, color: string) => {
    await createPinboard(name, color);
    setShowCreatePinboard(false);
  }, [createPinboard]);

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

      // Don't handle navigation keys when search is focused
      if (isSearchFocused) return;

      switch (e.key) {
        case " ": // Space — Quick Look preview
          e.preventDefault();
          if (displayClips.length > 0 && displayClips[selectedIndex]) {
            setShowPreview((prev) => !prev);
          }
          break;
        case "Escape":
          e.preventDefault();
          if (showPreview) {
            setShowPreview(false);
          } else if (isSearching) {
            handleClearSearch();
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
          if (e.shiftKey) {
            pastePlainSelected();
          } else {
            pasteSelected();
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
            const tabs: TabView[] = ["history", "pinboards", "snippets"];
            const idx = tabs.indexOf(prev);
            return tabs[(idx + 1) % tabs.length];
          });
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [displayClips, selectedIndex, pasteSelected, pastePlainSelected, deleteSelected, toggleFavoriteSelected, showPreview, showEditor, isSearching, handleClearSearch]);

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

  return (
    <div className="flex h-screen flex-col bg-surface-bg text-text-primary select-none">
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
            className={`rounded-md px-3 py-1 text-sm font-medium capitalize transition-colors ${
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
          className={`relative rounded-md px-3 py-1 text-sm font-medium transition-colors ${
            showPasteStack
              ? "bg-surface-hover text-text-primary"
              : "text-text-muted hover:text-text-secondary"
          }`}
        >
          Stack
          {pasteStackActive && (
            <span className="absolute -top-1 -right-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-green-600 px-1 text-[10px] font-bold text-white">
              {pasteStackCount}
            </span>
          )}
        </button>
        <span className="text-xs text-text-muted" aria-live="polite">
          {displayClips.length > 0
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
          onSelect={setSelectedIndex}
          onPaste={pasteSelected}
          loading={displayLoading}
          containerRef={containerRef}
          onLoadMore={isSearching ? undefined : loadMoreClips}
          hasMore={isSearching ? false : hasMore}
          loadingMore={isSearching ? false : loadingMore}
        />
      ) : activeTab === "pinboards" ? (
        <PinboardView
          pinboards={pinboards}
          onReload={reloadPinboards}
          onCreatePinboard={createPinboard}
          onUpdatePinboard={updatePinboard}
          onDeletePinboard={deletePinboard}
        />
      ) : (
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
      )}
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

      {/* Footer with keyboard hints */}
      <div
        className="flex items-center gap-4 border-t border-border-default px-4 py-1.5 text-xs text-text-muted"
        role="toolbar"
        aria-label="Keyboard shortcuts"
      >
        <span>←→ Navigate</span>
        <span>Enter Paste</span>
        <span>⇧Enter Plain</span>
        <span>Space Preview</span>
        <span>F Fav</span>
        <span>/ Search</span>
        <span>⌃E Edit</span>
        <span>⌃P Pin</span>
        <span>Del Remove</span>
        <span>Tab Views</span>
        <span>Esc Close</span>
      </div>
    </div>
  );
}

export default App;
