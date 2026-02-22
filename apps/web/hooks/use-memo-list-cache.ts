import { useEffect, useState } from "react";
import type { AuthUser as User } from "@supabase/supabase-js";
import type { SWRInfiniteKeyedMutator } from "swr/infinite";
import type { Memo } from "@/lib/types";
import {
  readMemoCache,
  sanitizeMemosForCache,
  writeMemoCache,
} from "@/lib/memo-cache";

type ResolvePages = (pages: Memo[][] | undefined) => Memo[][];

export function useMemoListCacheSeed(resolvedUser: User | null) {
  const [cachedMemos, setCachedMemos] = useState<Memo[] | undefined>(undefined);
  const [hasHydrated, setHasHydrated] = useState(false);

  useEffect(() => {
    setHasHydrated(true);
  }, []);

  useEffect(() => {
    if (!hasHydrated) return;
    if (!resolvedUser) {
      setCachedMemos(undefined);
      return;
    }
    const memoCache = readMemoCache(resolvedUser.id);
    setCachedMemos(memoCache?.memos);
  }, [hasHydrated, resolvedUser]);

  return { cachedMemos, hasHydrated };
}

interface UseMemoListCacheWriterOptions {
  resolvedUser: User | null;
  pages: Memo[][];
  mutate: SWRInfiniteKeyedMutator<Memo[][]>;
  resolvePages: ResolvePages;
}

export function useMemoListCacheWriter({
  resolvedUser,
  pages,
  mutate,
  resolvePages,
}: UseMemoListCacheWriterOptions) {
  useEffect(() => {
    if (!resolvedUser) return;
    if (pages.length === 0) return;
    const memoCache = sanitizeMemosForCache(pages[0] ?? []);
    writeMemoCache(resolvedUser.id, memoCache);
  }, [pages, resolvedUser]);

  useEffect(() => {
    if (!resolvedUser) return;
    const handleOfflineCreated = (event: Event) => {
      const detail = (event as CustomEvent<{ memo?: Memo }>).detail;
      const memo = detail?.memo;
      if (!memo || memo.user_id !== resolvedUser.id) return;
      mutate(
        (current) => {
          const basePages = resolvePages(current);
          const exists = basePages.some((page) =>
            page.some((item) => item.id === memo.id),
          );
          if (exists) return basePages;
          if (basePages.length === 0) return [[memo]];
          return [[memo, ...basePages[0]], ...basePages.slice(1)];
        },
        { revalidate: false },
      );

      const cacheRecord = readMemoCache(resolvedUser.id);
      const cachedMemos = cacheRecord?.memos ?? [];
      const nextCached = sanitizeMemosForCache([
        memo,
        ...cachedMemos.filter((item) => item.id !== memo.id),
      ]);
      writeMemoCache(resolvedUser.id, nextCached);
    };

    window.addEventListener("memo-offline-created", handleOfflineCreated);
    return () => {
      window.removeEventListener("memo-offline-created", handleOfflineCreated);
    };
  }, [mutate, resolvePages, resolvedUser]);
}
