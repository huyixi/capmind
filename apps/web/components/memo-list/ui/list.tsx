"use client";

import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { useWindowVirtualizer } from "@tanstack/react-virtual";
import { MemoCard } from "@/components/memo-card/ui/card";
import { Memo } from "@/lib/types";
import { MemoListEmptyState } from "./empty-state";

interface MemoListProps {
  memos: Memo[];
  onEdit: (memo: Memo) => void;
  onDelete: (
    id: string,
    images: string[],
    expectedVersion: string,
  ) => Promise<void>;
  onRestore?: (memo: Memo) => Promise<boolean>;
  isTrash?: boolean;
  isOnline: boolean;
  emptyTitle?: string;
  emptyDescription?: string;
  emptyAction?: ReactNode;
}

export function MemoList({
  memos,
  onEdit,
  onDelete,
  onRestore,
  isTrash = false,
  isOnline,
  emptyTitle,
  emptyDescription,
  emptyAction,
}: MemoListProps) {
  const listRef = useRef<HTMLDivElement | null>(null);
  const [scrollMargin, setScrollMargin] = useState(0);

  useLayoutEffect(() => {
    const updateScrollMargin = () => {
      if (!listRef.current) return;
      const rect = listRef.current.getBoundingClientRect();
      setScrollMargin(rect.top + window.scrollY);
    };

    updateScrollMargin();
    window.addEventListener("resize", updateScrollMargin);
    return () => window.removeEventListener("resize", updateScrollMargin);
  }, []);

  const virtualizer = useWindowVirtualizer({
    count: memos.length,
    estimateSize: () => 120,
    overscan: 6,
    scrollMargin,
    getItemKey: (index) => memos[index]?.clientId ?? memos[index]?.id ?? index,
  });
  const virtualItems = virtualizer.getVirtualItems();

  if (memos.length === 0) {
    return (
      <MemoListEmptyState
        isTrash={isTrash}
        title={emptyTitle}
        description={emptyDescription}
        action={emptyAction}
      />
    );
  }

  return (
    <div ref={listRef} className="w-full border-x">
      <div
        className="relative w-full"
        style={{ height: virtualizer.getTotalSize() }}
      >
        {virtualItems.map((virtualRow) => {
          const memo = memos[virtualRow.index];
          if (!memo) return null;
          return (
            <div
              key={virtualRow.key}
              ref={virtualizer.measureElement}
              data-index={virtualRow.index}
              className="absolute left-0 top-0 w-full"
              style={{
                transform: `translateY(${virtualRow.start - scrollMargin}px)`,
              }}
            >
              <MemoCard
                memo={memo}
                onEdit={onEdit}
                onDelete={onDelete}
                onRestore={onRestore}
                isTrash={isTrash}
                isOnline={isOnline}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
