import type { SupabaseClient } from "@supabase/supabase-js";

import { MEMO_IMAGES_BUCKET, MEMO_IMAGE_URL_TTL_SECONDS } from "@/lib/memo-constants";
import { normalizeMemoVersion } from "@/lib/memo-utils";
import { createSignedImageUrls } from "@/lib/supabase/storage";
import type { Memo } from "@/lib/types";

export type MemoImageRow = { url: string; sort_order: number };
export type MemoImageCountRow = { count: number };

export type MemoRow = {
  id: string;
  user_id: string;
  text: string;
  created_at: string;
  updated_at: string;
  version: string | number;
  deleted_at: string | null;
  memo_images?: MemoImageRow[] | MemoImageCountRow[];
};

export const MEMO_BASE_SELECT_FIELDS =
  "id, user_id, text, created_at, updated_at, version, deleted_at";
export const MEMO_BASE_WITH_IMAGE_COUNT_SELECT_FIELDS =
  `${MEMO_BASE_SELECT_FIELDS}, memo_images(count)`;
export const MEMO_SELECT_FIELDS =
  `${MEMO_BASE_SELECT_FIELDS}, memo_images(url, sort_order)`;

type BuildMemosOptions = {
  resolveImages?: boolean;
  hasImagesById?: Set<string>;
};

const isMemoImageRow = (
  row: MemoImageRow | MemoImageCountRow,
): row is MemoImageRow => typeof (row as MemoImageRow).url === "string";

const getMemoImageCount = (
  rows: (MemoImageRow | MemoImageCountRow)[] | undefined,
): number | undefined => {
  if (!rows) return undefined;
  if (rows.length === 0) return 0;
  const hasCount = rows.some((row) => "count" in row);
  if (!hasCount) return rows.length;
  return rows.reduce((sum, row) => {
    const count = (row as MemoImageCountRow).count;
    return Number.isFinite(count) ? sum + count : sum;
  }, 0);
};

export async function buildMemosFromRows(
  supabase: SupabaseClient,
  memoRows: MemoRow[],
  options: BuildMemosOptions = {},
): Promise<Memo[]> {
  const resolveImages = options.resolveImages ?? true;
  const hasImagesById = options.hasImagesById;
  const urlCounts: number[] = [];
  const rawImageUrls = memoRows.flatMap((memo) => {
    const urls =
      memo.memo_images?.filter(isMemoImageRow).map((image) => image.url) ?? [];
    urlCounts.push(urls.length);
    return urls;
  });
  const resolvedUrls =
    resolveImages && rawImageUrls.length > 0
      ? await createSignedImageUrls(
          supabase,
          MEMO_IMAGES_BUCKET,
          rawImageUrls,
          MEMO_IMAGE_URL_TTL_SECONDS,
        )
      : rawImageUrls;

  let cursor = 0;
  return memoRows.map((memo, index) => {
    const { memo_images, ...rest } = memo;
    const urlCount = urlCounts[index] ?? 0;
    const images = resolvedUrls.slice(cursor, cursor + urlCount);
    cursor += urlCount;
    const imageCount = getMemoImageCount(memo_images);
    const hasImages =
      hasImagesById?.has(rest.id) ??
      (memo_images ? (imageCount ?? 0) > 0 : undefined);
    return {
      ...rest,
      version: normalizeMemoVersion(rest.version),
      images,
      hasImages,
      imageCount,
    };
  });
}

export async function buildMemoFromRow(
  supabase: SupabaseClient,
  memoRow: MemoRow,
): Promise<Memo> {
  const [memo] = await buildMemosFromRows(supabase, [memoRow]);
  return memo;
}
