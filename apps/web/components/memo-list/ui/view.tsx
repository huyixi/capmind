"use client";

import type { ReactNode, RefObject } from "react";
import { Header } from "@/components/header";
import { MemoListSection } from "./section";
import { MemoListSearchDialog } from "./search-dialog";
import type { Memo } from "@/lib/types";
import type { AuthUser as User } from "@supabase/supabase-js";

interface MemoListViewProps {
  user: User | null;
  isRefreshing: boolean;
  isSyncing: boolean;
  isTrashActive: boolean;
  searchDisplayValue: string;
  onRefresh: () => void;
  onToggleTrash: () => void;
  onSearchOpen: () => void;
  onClearSearch: () => void;
  memos: Memo[];
  isLoadingInitial: boolean;
  isReachingEnd: boolean;
  isValidating: boolean;
  isOnline: boolean;
  error?: Error;
  onRetry?: () => void;
  onEdit: (memo: Memo) => void;
  onDelete: (
    id: string,
    images: string[],
    expectedVersion: string,
  ) => Promise<void>;
  onRestore?: (memo: Memo) => Promise<boolean>;
  loadMoreRef: RefObject<HTMLDivElement | null>;
  emptyState?: {
    title: string;
    description: string;
    action?: ReactNode;
  };
  isSearchOpen: boolean;
  onSearchOpenChange: (open: boolean) => void;
  searchQuery: string;
  onSearchApplyQuery: (value: string) => void;
}

export function MemoListView({
  user,
  isRefreshing,
  isSyncing,
  isTrashActive,
  searchDisplayValue,
  onRefresh,
  onToggleTrash,
  onSearchOpen,
  onClearSearch,
  memos,
  isLoadingInitial,
  isReachingEnd,
  isValidating,
  isOnline,
  error,
  onRetry,
  onEdit,
  onDelete,
  onRestore,
  loadMoreRef,
  emptyState,
  isSearchOpen,
  onSearchOpenChange,
  searchQuery,
  onSearchApplyQuery,
}: MemoListViewProps) {
  return (
    <>
      <Header
        user={user}
        onRefresh={onRefresh}
        onToggleTrash={onToggleTrash}
        isTrashActive={isTrashActive}
        onSearchOpen={onSearchOpen}
        onClearSearch={onClearSearch}
        isRefreshing={isRefreshing}
        isSyncing={isSyncing}
        searchQuery={searchDisplayValue}
      />

      <MemoListSection
        memos={memos}
        isLoadingInitial={isLoadingInitial}
        isReachingEnd={isReachingEnd}
        isValidating={isValidating}
        isOnline={isOnline}
        error={error}
        onRetry={onRetry}
        onEdit={onEdit}
        onDelete={onDelete}
        onRestore={onRestore}
        isTrash={isTrashActive}
        loadMoreRef={loadMoreRef}
        emptyState={emptyState}
      />

      {user ? (
        <MemoListSearchDialog
          open={isSearchOpen}
          onOpenChange={onSearchOpenChange}
          appliedQuery={searchQuery}
          onApplyQuery={onSearchApplyQuery}
        />
      ) : null}
    </>
  );
}
