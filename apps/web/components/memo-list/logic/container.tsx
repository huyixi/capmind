"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { MemoListView } from "../ui/view";
import { Memo } from "@/lib/types";
import { type AuthUser as User } from "@supabase/supabase-js";
import { useMemoPagination } from "@/hooks/use-memo-pagination";
import { useOnlineStatus } from "@/hooks/use-online-status";
import { useResolvedUser } from "@/hooks/use-resolved-user";
import {
  useMemoListCacheSeed,
  useMemoListCacheWriter,
} from "@/hooks/use-memo-list-cache";
import { useMemoListSearch } from "@/hooks/use-memo-list-search";
import { preloadMemoListSearchDialog } from "../ui/search-dialog";
import { useMemoSync } from "@/hooks/use-memo-sync";
import { useMemoMutations } from "@/hooks/use-memo-mutations";

export type MemoComposerActions = {
  handleSubmit: (text: string, images: File[]) => void;
  handleUpdate: (payload: {
    id: string;
    text: string;
    expectedVersion: string;
    existingImageUrls?: string[];
    newImages?: File[];
  }) => Promise<void>;
};

export type MemoSearchActions = {
  openSearch: () => void;
  closeSearch: () => void;
};

interface MemoListContainerProps {
  initialUser: User | null;
  initialMemos?: Memo[];
  onEdit: (memo: Memo) => void;
  onRegisterComposerActions?: (actions: MemoComposerActions | null) => void;
  onRegisterSearchActions?: (actions: MemoSearchActions | null) => void;
  onResetComposer?: () => void;
}

const MIN_REFRESH_DURATION_MS = 600;

export function MemoListContainer({
  initialUser,
  initialMemos,
  onEdit,
  onRegisterComposerActions,
  onRegisterSearchActions,
  onResetComposer,
}: MemoListContainerProps) {
  const supabase = useMemo(() => createClient(), []);
  const { resolvedUser } = useResolvedUser(initialUser, supabase);
  const { cachedMemos } = useMemoListCacheSeed(resolvedUser);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const loadMoreRef = useRef<HTMLDivElement | null>(null);
  const isOnline = useOnlineStatus();
  const [isTrashView, setIsTrashView] = useState(false);

  const {
    memos,
    pages,
    resolvePages,
    fetchPage,
    size,
    setSize,
    mutate,
    isValidating,
    isLoadingInitial,
    isLoadingMore,
    isReachingEnd,
    error: memosError,
  } = useMemoPagination({
    initialUser: resolvedUser,
    initialMemos: initialMemos ?? cachedMemos,
    enabled: Boolean(resolvedUser),
  });

  const trashQuery = useMemo(() => ({ trash: "1" }), []);
  const {
    memos: trashMemos,
    resolvePages: resolveTrashPages,
    fetchPage: fetchTrashPage,
    size: trashSize,
    setSize: setTrashSize,
    mutate: mutateTrash,
    isValidating: isTrashValidating,
    isLoadingInitial: isTrashLoadingInitial,
    isLoadingMore: isTrashLoadingMore,
    isReachingEnd: isTrashReachingEnd,
    error: trashError,
  } = useMemoPagination({
    initialUser: resolvedUser,
    query: trashQuery,
    enabled: Boolean(resolvedUser) && isTrashView,
  });

  const {
    isSearchOpen,
    setIsSearchOpen,
    searchQuery,
    setSearchQuery,
    isSearchActive,
    searchResults,
    isSearching,
    searchError,
    openSearch,
    closeSearch,
    clearSearch,
    retrySearch,
  } = useMemoListSearch({
    resolvedUser,
    isTrashView,
    setIsTrashView,
    isOnline,
  });
  const pagingIsLoadingMore = isTrashView ? isTrashLoadingMore : isLoadingMore;
  const pagingIsReachingEnd = isTrashView ? isTrashReachingEnd : isReachingEnd;
  const pagingSetSize = isTrashView ? setTrashSize : setSize;

  const {
    cleanupOptimisticImages,
    fetchServerMemo,
    handleDelete,
    handleRestore,
    handleSubmit,
    handleUpdate,
    removeMemo,
    replaceMemo,
    uploadImages,
  } = useMemoMutations({
    initialUser: resolvedUser,
    isOnline,
    mutate,
    resolvePages,
    supabase,
  });

  useEffect(() => {
    if (!onRegisterComposerActions) return;
    if (!resolvedUser) {
      onRegisterComposerActions(null);
      return;
    }
    onRegisterComposerActions({ handleSubmit, handleUpdate });
    return () => onRegisterComposerActions(null);
  }, [handleSubmit, handleUpdate, onRegisterComposerActions, resolvedUser]);

  const { flushOutbox, isSyncing } = useMemoSync({
    initialUser: resolvedUser,
    isOnline,
    mutate,
    resolvePages,
    replaceMemo,
    removeMemo,
    fetchServerMemo,
    uploadImages,
    supabase,
    cleanupOptimisticImages,
  });

  useMemoListCacheWriter({
    resolvedUser,
    pages,
    mutate,
    resolvePages,
  });

  const baseMemos = isTrashView ? trashMemos : memos;
  const baseIsLoadingInitial = isTrashView
    ? isTrashLoadingInitial
    : isLoadingInitial;
  const baseIsReachingEnd = isTrashView ? isTrashReachingEnd : isReachingEnd;
  const baseIsValidating = isTrashView ? isTrashValidating : isValidating;
  const baseError = isTrashView ? trashError : memosError;
  const baseOnRetry = resolvedUser
    ? isTrashView
      ? () => mutateTrash()
      : () => mutate()
    : undefined;

  const listMemos = isSearchActive ? searchResults : baseMemos;
  const listIsLoadingInitial = isSearchActive
    ? isSearching
    : baseIsLoadingInitial;
  const listIsReachingEnd = isSearchActive ? true : baseIsReachingEnd;
  const listIsValidating = isSearchActive ? false : baseIsValidating;
  const listError =
    isSearchActive && searchError ? new Error(searchError) : baseError;
  const listOnRetry =
    isSearchActive && searchError ? retrySearch : baseOnRetry;
  const listEmptyState = isSearchActive
    ? isOnline
      ? {
          title: "No matching memos",
          description: "Try a different keyword or clear the filter.",
        }
      : {
          title: "Offline",
          description: "Connect to the internet to search.",
        }
    : undefined;

  const handleSearchOpen = useCallback(() => {
    preloadMemoListSearchDialog();
    openSearch();
  }, [openSearch]);

  useEffect(() => {
    if (!onRegisterSearchActions) return;
    onRegisterSearchActions({
      openSearch: handleSearchOpen,
      closeSearch,
    });
    return () => onRegisterSearchActions(null);
  }, [closeSearch, handleSearchOpen, onRegisterSearchActions]);

  useEffect(() => {
    if (!resolvedUser) return;
    if (isSearchActive) return;
    const target = loadMoreRef.current;
    if (!target) return;
    if (pagingIsReachingEnd) return;

    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (entry?.isIntersecting && !pagingIsLoadingMore) {
          pagingSetSize((current) => current + 1);
        }
      },
      { rootMargin: "800px" },
    );

    observer.observe(target);
    return () => observer.disconnect();
  }, [
    isSearchActive,
    pagingIsLoadingMore,
    pagingIsReachingEnd,
    pagingSetSize,
    resolvedUser,
  ]);

  const handleToggleTrash = useCallback(() => {
    setIsSearchOpen(false);
    onResetComposer?.();
    setIsTrashView((current) => !current);
  }, [onResetComposer, setIsSearchOpen]);

  const handleClearSearch = useCallback(() => {
    clearSearch();
  }, [clearSearch]);

  const handleTrashRestore = useCallback(
    async (memo: Memo): Promise<boolean> => {
      try {
        await mutateTrash(
          async (current) => {
            const basePages = resolveTrashPages(current);
            const ok = await handleRestore(memo);
            if (!ok) {
              throw new Error("Restore failed");
            }
            return removeMemo(basePages, memo.id);
          },
          {
            optimisticData: (current) =>
              removeMemo(resolveTrashPages(current), memo.id),
            rollbackOnError: true,
            revalidate: false,
          },
        );
        return true;
      } catch {
        // Rollback handled by SWR; no-op here.
        return false;
      }
    },
    [handleRestore, mutateTrash, removeMemo, resolveTrashPages],
  );

  const didWarmRefreshRef = useRef(false);
  const isCacheSeeded = initialMemos === undefined && cachedMemos !== undefined;
  useEffect(() => {
    if (!isCacheSeeded) return;
    if (!isOnline) return;
    if (didWarmRefreshRef.current) return;
    didWarmRefreshRef.current = true;
    void mutate();
  }, [isCacheSeeded, isOnline, mutate]);

  const handleRefresh = useCallback(async () => {
    if (!resolvedUser) return;
    if (!isOnline) return;
    const refreshFetchPage = isTrashView ? fetchTrashPage : fetchPage;
    const refreshMutate = isTrashView ? mutateTrash : mutate;
    const refreshSize = isTrashView ? trashSize : size;
    const refreshSetSize = isTrashView ? setTrashSize : setSize;
    const refreshStart = Date.now();
    setIsRefreshing(true);
    try {
      const result = await flushOutbox();
      if (result.hadError) return;
      if (refreshSize > 1) {
        await refreshSetSize(1);
      }
      try {
        const refreshedPage = await refreshFetchPage(["/api/memos", 0]);
        await refreshMutate([refreshedPage], { revalidate: false });
      } catch (error) {
        console.error("Error refreshing memos:", error);
      }
    } finally {
      const elapsed = Date.now() - refreshStart;
      if (elapsed < MIN_REFRESH_DURATION_MS) {
        await new Promise((resolve) =>
          window.setTimeout(resolve, MIN_REFRESH_DURATION_MS - elapsed),
        );
      }
      setIsRefreshing(false);
    }
  }, [
    fetchPage,
    fetchTrashPage,
    flushOutbox,
    isOnline,
    isTrashView,
    mutate,
    mutateTrash,
    resolvedUser,
    setSize,
    setTrashSize,
    size,
    trashSize,
  ]);

  const searchDisplayValue = isTrashView ? "#trash" : searchQuery;

  return (
    <MemoListView
      user={resolvedUser}
      isRefreshing={isRefreshing}
      isSyncing={isSyncing}
      isTrashActive={isTrashView}
      searchDisplayValue={searchDisplayValue}
      onRefresh={handleRefresh}
      onToggleTrash={handleToggleTrash}
      onSearchOpen={handleSearchOpen}
      onClearSearch={handleClearSearch}
      memos={resolvedUser ? listMemos : []}
      isLoadingInitial={resolvedUser ? listIsLoadingInitial : false}
      isReachingEnd={resolvedUser ? listIsReachingEnd : true}
      isValidating={resolvedUser ? listIsValidating : false}
      isOnline={isOnline}
      error={resolvedUser ? listError : undefined}
      onRetry={resolvedUser ? listOnRetry : undefined}
      onEdit={onEdit}
      onDelete={handleDelete}
      onRestore={isTrashView ? handleTrashRestore : handleRestore}
      loadMoreRef={loadMoreRef}
      emptyState={listEmptyState}
      isSearchOpen={isSearchOpen}
      onSearchOpenChange={setIsSearchOpen}
      searchQuery={searchQuery}
      onSearchApplyQuery={setSearchQuery}
    />
  );
}
