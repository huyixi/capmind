"use client";

import { Loader2 } from "lucide-react";

export function MemoListSkeleton() {
  return (
    <div className="space-y-4 py-6">
      <div className="h-10 w-2/3 animate-pulse rounded-md bg-muted/70" />
      <div className="h-16 w-full animate-pulse rounded-md bg-muted/60" />
      <div className="h-24 w-full animate-pulse rounded-md bg-muted/60" />
      <div className="h-16 w-11/12 animate-pulse rounded-md bg-muted/60" />
      <div className="h-20 w-full animate-pulse rounded-md bg-muted/60" />
      <div className="flex items-center justify-center pt-2">
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    </div>
  );
}
