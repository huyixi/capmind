import { useEffect, useRef, useState } from "react";
import type { Memo } from "@/lib/types";

interface MemoSearchOptions {
  query: string;
  enabled: boolean;
  isOnline: boolean;
  requestId: number;
  delayMs?: number;
}

const DEFAULT_DELAY_MS = 200;

export function useMemoSearch({
  query,
  enabled,
  isOnline,
  requestId,
  delayMs = DEFAULT_DELAY_MS,
}: MemoSearchOptions) {
  const [results, setResults] = useState<Memo[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const lastQueryRef = useRef("");

  useEffect(() => {
    if (!enabled) {
      setResults([]);
      setError(null);
      setIsSearching(false);
      lastQueryRef.current = "";
      return;
    }

    if (!query) {
      setResults([]);
      setError(null);
      setIsSearching(false);
      lastQueryRef.current = "";
      return;
    }

    if (!isOnline) {
      setResults([]);
      setError(null);
      setIsSearching(false);
      return;
    }

    const controller = new AbortController();
    const searchValue = query;
    lastQueryRef.current = searchValue;
    setError(null);
    setIsSearching(true);
    setResults([]);

    const timeoutId = window.setTimeout(async () => {
      try {
        const params = new URLSearchParams();
        params.set("q", searchValue);
        const response = await fetch(`/api/memos/search?${params.toString()}`, {
          credentials: "include",
          cache: "no-store",
          headers: { "cache-control": "no-cache" },
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(response.statusText);
        }

        const payload = await response.json();
        const fetchedMemos = (payload.memos ?? []) as Memo[];
        if (lastQueryRef.current === searchValue) {
          setResults(fetchedMemos);
        }
      } catch (fetchError) {
        if ((fetchError as Error).name === "AbortError") return;
        console.error("Error searching memos:", fetchError);
        setError("Search failed. Please try again.");
      } finally {
        if (!controller.signal.aborted) {
          setIsSearching(false);
        }
      }
    }, delayMs);

    return () => {
      window.clearTimeout(timeoutId);
      controller.abort();
    };
  }, [delayMs, enabled, isOnline, query, requestId]);

  return {
    results,
    isSearching,
    error,
  };
}
