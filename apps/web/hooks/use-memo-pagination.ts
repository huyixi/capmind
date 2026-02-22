import { useCallback, useMemo } from "react";
import useSWRInfinite from "swr/infinite";
import { useSWRConfig } from "swr";
import { MEMOS_PAGE_SIZE } from "@/lib/memo-constants";
import { normalizeMemoVersion } from "@/lib/memo-utils";
import { Memo } from "@/lib/types";
import { type AuthUser as User } from "@supabase/supabase-js";

type FetchPageArgs = [string, number] | string;
type FetchPage = (...args: FetchPageArgs[]) => Promise<Memo[]>;

interface MemoPaginationOptions {
  initialUser: User | null;
  initialMemos?: Memo[];
  baseUrl?: string;
  query?: Record<string, string | number | boolean | null | undefined>;
  enabled?: boolean;
  prefetchNextPage?: boolean;
  onAuthError?: (status: number) => void;
  onAuthSuccess?: () => void;
}

export function useMemoPagination({
  initialMemos,
  baseUrl = "/api/memos",
  query,
  enabled = true,
  prefetchNextPage = true,
  onAuthError,
  onAuthSuccess,
}: MemoPaginationOptions) {
  const { mutate: globalMutate } = useSWRConfig();
  const hasInitialMemos = initialMemos !== undefined;
  const canUseInitialMemos = hasInitialMemos;
  const initialPages = useMemo(
    () => (canUseInitialMemos ? [initialMemos ?? []] : []),
    [canUseInitialMemos, initialMemos],
  );
  const resolvePages = useCallback(
    (pages: Memo[][] | undefined) => pages ?? initialPages,
    [initialPages],
  );

  const queryString = useMemo(() => {
    if (!query) return "";
    const entries = Object.entries(query)
      .filter(([, value]) => value !== undefined && value !== null)
      .map(([key, value]) => [key, String(value)] as const)
      .sort(([left], [right]) => left.localeCompare(right));
    if (entries.length === 0) return "";
    const params = new URLSearchParams();
    entries.forEach(([key, value]) => {
      params.set(key, value);
    });
    return params.toString();
  }, [query]);

  const fetchPage: FetchPage = useCallback(async (...args) => {
    let url: string | undefined;
    let pageIndex: number | undefined;

    if (Array.isArray(args[0])) {
      [url, pageIndex] = args[0] as [string, number];
    } else {
      [url, pageIndex] = args as [string, number];
    }

    const safeUrl = url ?? baseUrl;
    const safePageIndex = Number.isFinite(pageIndex) ? pageIndex : 0;
    const params = new URLSearchParams(queryString);
    params.set("page", safePageIndex.toString());
    params.set("pageSize", MEMOS_PAGE_SIZE.toString());
    if (prefetchNextPage) {
      params.set("prefetch", "1");
    }

    const response = await fetch(`${safeUrl}?${params.toString()}`, {
      credentials: "include",
      cache: "no-store",
      headers: { "cache-control": "no-cache" },
    });
    if (response.status === 401 || response.status === 403) {
      // Treat auth failures as empty results to avoid retry loops.
      onAuthError?.(response.status);
      return [];
    }
    if (!response.ok) {
      console.error("Error fetching memos:", response.statusText);
      throw new Error("Failed to fetch memos");
    }
    onAuthSuccess?.();

    const payload = await response.json();
    const memos = (payload.memos ?? []) as (Memo & {
      version: string | number;
      serverVersion?: string | number;
    })[];
    const normalized = memos.map((memo) => ({
      ...memo,
      version: normalizeMemoVersion(memo.version),
      serverVersion: normalizeMemoVersion(memo.serverVersion ?? memo.version),
      hasConflict: false,
      conflictServerMemo: undefined,
    }));

    if (prefetchNextPage && Array.isArray(payload.prefetched)) {
      const prefetched = payload.prefetched as (Memo & {
        version: string | number;
        serverVersion?: string | number;
      })[];
      if (prefetched.length > 0) {
        const normalizedPrefetch = prefetched.map((memo) => ({
          ...memo,
          version: normalizeMemoVersion(memo.version),
          serverVersion: normalizeMemoVersion(memo.serverVersion ?? memo.version),
          hasConflict: false,
          conflictServerMemo: undefined,
        }));
        const nextKey: [string, number, string] = [
          baseUrl,
          safePageIndex + 1,
          queryString,
        ];
        void globalMutate(nextKey, normalizedPrefetch, { revalidate: false });
      }
    }

    return normalized;
  }, [
    baseUrl,
    globalMutate,
    onAuthError,
    onAuthSuccess,
    prefetchNextPage,
    queryString,
  ]);

  const getKey = useCallback(
    (pageIndex: number, previousPageData: Memo[] | null) => {
      if (!enabled) return null;
      if (previousPageData && previousPageData.length < MEMOS_PAGE_SIZE) {
        return null;
      }
      return [baseUrl, pageIndex, queryString];
    },
    [baseUrl, enabled, queryString],
  );

  const { data, size, setSize, mutate, error, isValidating } =
    useSWRInfinite<Memo[]>(getKey, fetchPage, {
      fallbackData: canUseInitialMemos ? [initialMemos ?? []] : undefined,
      revalidateOnFocus: false,
      revalidateFirstPage: !canUseInitialMemos,
    });

  const pages = data ?? initialPages;
  const memos = pages.flat();
  const isLoadingInitial = !data && !error;
  const isLoadingMore = isLoadingInitial || size > pages.length;
  const isEmpty = pages.length > 0 && pages[0].length === 0;
  const lastPageSize = pages.length > 0 ? pages[pages.length - 1].length : null;
  const isReachingEnd =
    isEmpty || (lastPageSize !== null && lastPageSize < MEMOS_PAGE_SIZE);

  return {
    memos,
    pages,
    resolvePages,
    fetchPage,
    size,
    setSize,
    mutate,
    error,
    isValidating,
    isLoadingInitial,
    isLoadingMore,
    isReachingEnd,
  };
}
