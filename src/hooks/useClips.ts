import { useState } from "react";

interface Clip {
  id: string;
  contentType: string;
  textContent: string;
  sourceApp: string;
  createdAt: string;
}

export function useClips() {
  const [clips] = useState<Clip[]>([]);
  const [loading] = useState(false);

  return { clips, loading };
}
