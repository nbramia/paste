import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SnippetData {
  id: string;
  abbreviation: string;
  name: string;
  content: string;
  content_type: string;
  group_id: string | null;
  description: string | null;
  use_count: number;
  created_at: string;
  updated_at: string;
}

export interface SnippetGroupData {
  id: string;
  name: string;
  position: number;
  created_at: string;
}

export function useSnippets() {
  const [snippets, setSnippets] = useState<SnippetData[]>([]);
  const [groups, setGroups] = useState<SnippetGroupData[]>([]);
  const [loading, setLoading] = useState(true);

  const loadAll = useCallback(async () => {
    try {
      setLoading(true);
      const [snips, grps] = await Promise.all([
        invoke<SnippetData[]>("list_snippets", {}),
        invoke<SnippetGroupData[]>("list_snippet_groups"),
      ]);
      setSnippets(snips);
      setGroups(grps);
    } catch (err) {
      console.error("Failed to load snippets:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadAll();
  }, [loadAll]);

  const createSnippet = useCallback(
    async (data: {
      abbreviation: string;
      name: string;
      content: string;
      contentType: string;
      groupId: string | null;
      description: string | null;
    }) => {
      await invoke("create_snippet", {
        abbreviation: data.abbreviation,
        name: data.name,
        content: data.content,
        contentType: data.contentType,
        groupId: data.groupId,
        description: data.description,
      });
      await loadAll();
    },
    [loadAll],
  );

  const updateSnippet = useCallback(
    async (
      id: string,
      data: {
        abbreviation: string;
        name: string;
        content: string;
        contentType: string;
        groupId: string | null;
        description: string | null;
      },
    ) => {
      await invoke("update_snippet", {
        id,
        abbreviation: data.abbreviation,
        name: data.name,
        content: data.content,
        contentType: data.contentType,
        groupId: data.groupId,
        description: data.description,
      });
      await loadAll();
    },
    [loadAll],
  );

  const deleteSnippet = useCallback(
    async (id: string) => {
      await invoke("delete_snippet", { id });
      await loadAll();
    },
    [loadAll],
  );

  const createGroup = useCallback(
    async (name: string) => {
      await invoke("create_snippet_group", { name });
      await loadAll();
    },
    [loadAll],
  );

  const deleteGroup = useCallback(
    async (id: string) => {
      await invoke("delete_snippet_group", { id });
      await loadAll();
    },
    [loadAll],
  );

  return {
    snippets,
    groups,
    loading,
    reload: loadAll,
    createSnippet,
    updateSnippet,
    deleteSnippet,
    createGroup,
    deleteGroup,
  };
}
