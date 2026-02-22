"use client";

import { useCallback, useEffect, useState } from "react";

export const RECENT_SEARCH_STORAGE_KEY = "cap.memo.search.history";
export const DEFAULT_MAX_RECENT = 8;

const loadRecentSearchesFromStorage = (storageKey: string) => {
  if (typeof window === "undefined") return [];
  try {
    const stored = window.localStorage.getItem(storageKey);
    if (!stored) return [];
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((value) => typeof value === "string");
  } catch {
    return [];
  }
};

interface UseRecentSearchesOptions {
  storageKey?: string;
  maxRecent?: number;
}

export function useRecentSearches(
  options: UseRecentSearchesOptions = {},
) {
  const {
    storageKey = RECENT_SEARCH_STORAGE_KEY,
    maxRecent = DEFAULT_MAX_RECENT,
  } = options;

  const [recentSearches, setRecentSearches] = useState<string[]>([]);

  const reloadRecentSearches = useCallback(() => {
    setRecentSearches(loadRecentSearchesFromStorage(storageKey));
  }, [storageKey]);

  const updateRecentSearches = useCallback(
    (updater: (items: string[]) => string[]) => {
      setRecentSearches((current) => {
        const next = updater(current);
        if (typeof window !== "undefined") {
          try {
            window.localStorage.setItem(storageKey, JSON.stringify(next));
          } catch {
            // Ignore storage errors.
          }
        }
        return next;
      });
    },
    [storageKey],
  );

  const addRecentSearch = useCallback(
    (value: string) => {
      const trimmed = value.trim();
      if (!trimmed) return;
      updateRecentSearches((current) =>
        [
          trimmed,
          ...current.filter(
            (item) => item.toLowerCase() !== trimmed.toLowerCase(),
          ),
        ].slice(0, maxRecent),
      );
    },
    [maxRecent, updateRecentSearches],
  );

  const removeRecentSearch = useCallback(
    (value: string) => {
      updateRecentSearches((current) =>
        current.filter((item) => item !== value),
      );
    },
    [updateRecentSearches],
  );

  useEffect(() => {
    reloadRecentSearches();
  }, [reloadRecentSearches]);

  return {
    recentSearches,
    addRecentSearch,
    removeRecentSearch,
    reloadRecentSearches,
  };
}
