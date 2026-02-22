import { useCallback, useMemo, useState } from "react";
import type { AuthUser as User } from "@supabase/supabase-js";
import { useMemoSearch } from "@/hooks/use-memo-search";

interface UseMemoListSearchOptions {
  resolvedUser: User | null;
  isTrashView: boolean;
  setIsTrashView: (value: boolean | ((prev: boolean) => boolean)) => void;
  isOnline: boolean;
}

export function useMemoListSearch({
  resolvedUser,
  isTrashView,
  setIsTrashView,
  isOnline,
}: UseMemoListSearchOptions) {
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchRequestId, setSearchRequestId] = useState(0);
  const trimmedSearchQuery = useMemo(() => searchQuery.trim(), [searchQuery]);
  const isSearchEnabled = Boolean(resolvedUser) && !isTrashView;
  const isSearchActive = !isTrashView && Boolean(trimmedSearchQuery);

  const { results: searchResults, isSearching, error: searchError } =
    useMemoSearch({
      query: trimmedSearchQuery,
      enabled: isSearchEnabled,
      isOnline,
      requestId: searchRequestId,
    });

  const openSearch = useCallback(() => {
    if (!resolvedUser) return;
    if (isTrashView) {
      setIsTrashView(false);
    }
    setIsSearchOpen(true);
  }, [isTrashView, resolvedUser, setIsTrashView]);

  const closeSearch = useCallback(() => {
    setIsSearchOpen(false);
  }, []);

  const clearSearch = useCallback(() => {
    setIsSearchOpen(false);
    if (isTrashView) {
      setIsTrashView(false);
      return;
    }
    setSearchQuery("");
  }, [isTrashView, setIsTrashView]);

  const retrySearch = useCallback(() => {
    setSearchRequestId((current) => current + 1);
  }, []);

  return {
    isSearchOpen,
    setIsSearchOpen,
    searchQuery,
    setSearchQuery,
    trimmedSearchQuery,
    isSearchEnabled,
    isSearchActive,
    searchResults,
    isSearching,
    searchError,
    openSearch,
    closeSearch,
    clearSearch,
    retrySearch,
  };
}
