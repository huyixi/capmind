"use client";

import { type ReactNode, type RefObject } from "react";
import type { Memo } from "@/lib/types";
import { MemoList } from "./list";
import { MemoListErrorState } from "./section-error";
import { MemoListFooter } from "./section-footer";
import { MemoListSkeleton } from "./section-skeleton";

interface MemoListSectionProps {
  memos: Memo[];
  isLoadingInitial: boolean;
  isReachingEnd: boolean;
  isValidating: boolean;
  isOnline: boolean;
  error?: Error;
  onRetry?: () => void;
  emptyState?: {
    title: string;
    description: string;
    action?: ReactNode;
  };
  onEdit: (memo: Memo) => void;
  onDelete: (
    id: string,
    images: string[],
    expectedVersion: string,
  ) => Promise<void>;
  onRestore?: (memo: Memo) => Promise<boolean>;
  isTrash: boolean;
  loadMoreRef: RefObject<HTMLDivElement | null>;
}

export function MemoListSection({
  memos,
  isLoadingInitial,
  isReachingEnd,
  isValidating,
  isOnline,
  error,
  onRetry,
  emptyState,
  onEdit,
  onDelete,
  onRestore,
  isTrash,
  loadMoreRef,
}: MemoListSectionProps) {
  if (isLoadingInitial) {
    return (
      <main className="flex-1 mx-auto w-full max-w-xl mb-20">
        <MemoListSkeleton />
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 mx-auto w-full max-w-xl mb-20">
        <MemoListErrorState onRetry={onRetry} />
      </main>
    );
  }

  return (
    <main className="flex-1 mx-auto w-full max-w-xl mb-20">
      <MemoList
        memos={memos}
        onEdit={onEdit}
        onDelete={onDelete}
        onRestore={onRestore}
        isTrash={isTrash}
        isOnline={isOnline}
        emptyTitle={emptyState?.title}
        emptyDescription={emptyState?.description}
        emptyAction={emptyState?.action}
      />
      <MemoListFooter
        loadMoreRef={loadMoreRef}
        isReachingEnd={isReachingEnd}
        isValidating={isValidating}
      />
    </main>
  );
}
