import { useState } from "react";

interface Clip {
  id: string;
  contentType: string;
  textContent: string;
}

export function useSearch() {
  const [query, setQuery] = useState("");
  const [results] = useState<Clip[]>([]);

  return { query, setQuery, results };
}
