"use client";

import type { RefObject } from "react";
import { Loader2 } from "lucide-react";

interface MemoListFooterProps {
  loadMoreRef: RefObject<HTMLDivElement | null>;
  isReachingEnd: boolean;
  isValidating: boolean;
}

export function MemoListFooter({
  loadMoreRef,
  isReachingEnd,
  isValidating,
}: MemoListFooterProps) {
  return (
    <>
      <div ref={loadMoreRef} className="h-8" />
      {!isReachingEnd && isValidating ? (
        <div className="flex items-center justify-center py-4">
          <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        </div>
      ) : null}
    </>
  );
}
