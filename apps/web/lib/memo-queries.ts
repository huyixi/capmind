import type { SupabaseClient } from "@supabase/supabase-js";

import {
  buildMemosFromRows,
  MEMO_BASE_WITH_IMAGE_COUNT_SELECT_FIELDS,
  type MemoRow,
} from "@/lib/memos";
import type { Memo } from "@/lib/types";

type FetchMemoPageParams = {
  supabase: SupabaseClient;
  userId: string;
  page: number;
  pageSize: number;
  isTrashView: boolean;
};

type FetchMemoPageResult = {
  memos: Memo[];
  error: unknown | null;
  timings: {
    memosMs: number;
    memoImagesMs: number;
    buildMs: number;
    totalMs: number;
  };
};

export async function fetchMemoPage({
  supabase,
  userId,
  page,
  pageSize,
  isTrashView,
}: FetchMemoPageParams): Promise<FetchMemoPageResult> {
  const totalStart = Date.now();
  const from = page * pageSize;
  const to = from + pageSize - 1;

  let memoQuery = supabase
    .from("memos")
    .select(MEMO_BASE_WITH_IMAGE_COUNT_SELECT_FIELDS)
    .eq("user_id", userId)
    .order(isTrashView ? "deleted_at" : "created_at", { ascending: false });

  memoQuery = isTrashView
    ? memoQuery.not("deleted_at", "is", null)
    : memoQuery.is("deleted_at", null);

  memoQuery = memoQuery.range(from, to);

  const memosStart = Date.now();
  const { data, error } = await memoQuery;
  const memosMs = Date.now() - memosStart;
  if (error) {
    return {
      memos: [],
      error,
      timings: {
        memosMs,
        memoImagesMs: 0,
        buildMs: 0,
        totalMs: Date.now() - totalStart,
      },
    };
  }

  const memoRows = (data ?? []) as MemoRow[];
  const buildStart = Date.now();
  const memos = await buildMemosFromRows(supabase, memoRows, {
    resolveImages: false,
  });
  const buildMs = Date.now() - buildStart;

  return {
    memos,
    error: null,
    timings: {
      memosMs,
      // memo_images(count) is fetched inline with the memos query.
      memoImagesMs: 0,
      buildMs,
      totalMs: Date.now() - totalStart,
    },
  };
}
