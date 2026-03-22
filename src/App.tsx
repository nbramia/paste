import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Filmstrip } from "./components/Filmstrip";
import { Search, type SearchFilters } from "./components/Search";
import { useSearch } from "./hooks/useSearch";

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
  const [clips, setClips] = useState<ClipData[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [activeTab, setActiveTab] = useState<TabView>("history");
  const [loading, setLoading] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const { results, isSearching, loading: searchLoading, search, clearSearch } = useSearch();

  // The clips to display: search results when searching, all clips otherwise
  const displayClips = isSearching ? results : clips;
  const displayLoading = isSearching ? searchLoading : loading;

  const loadClips = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<ClipData[]>("get_clips", {
        offset: 0,
        limit: 50,
      });
      setClips(result);
      setSelectedIndex(0);
    } catch (err) {
      console.error("Failed to load clips:", err);
    } finally {
      setLoading(false);
    }
  }, []);

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

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Check if search input is focused
      const isSearchFocused = document.activeElement === searchRef.current;

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
          pasteSelected();
          break;
        case "Delete":
        case "Backspace":
          e.preventDefault();
          deleteSelected();
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
  }, [displayClips, pasteSelected, deleteSelected]);

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
    <div className="flex h-screen flex-col bg-neutral-900 text-white select-none">
      {/* Tab bar */}
      <div className="flex items-center gap-1 border-b border-neutral-700 px-4 py-2">
        {(["history", "pinboards", "snippets"] as TabView[]).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`rounded-md px-3 py-1 text-sm font-medium capitalize transition-colors ${
              activeTab === tab
                ? "bg-neutral-700 text-white"
                : "text-neutral-400 hover:text-neutral-200"
            }`}
          >
            {tab}
          </button>
        ))}
        <div className="flex-1" />
        <span className="text-xs text-neutral-500">
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

      {/* Filmstrip content */}
      {activeTab === "history" ? (
        <Filmstrip
          clips={displayClips}
          selectedIndex={selectedIndex}
          onSelect={setSelectedIndex}
          onPaste={pasteSelected}
          loading={displayLoading}
          containerRef={containerRef}
        />
      ) : (
        <div className="flex flex-1 items-center justify-center text-neutral-500">
          {activeTab === "pinboards"
            ? "Pinboards (coming soon)"
            : "Snippets (coming soon)"}
        </div>
      )}

      {/* Footer with keyboard hints */}
      <div className="flex items-center gap-4 border-t border-neutral-700 px-4 py-1.5 text-xs text-neutral-500">
        <span>←→ Navigate</span>
        <span>Enter Paste</span>
        <span>/ Search</span>
        <span>Del Remove</span>
        <span>Tab Switch view</span>
        <span>Esc Close</span>
      </div>
    </div>
  );
}

export default App;
