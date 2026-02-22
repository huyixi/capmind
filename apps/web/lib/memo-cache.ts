import type { Memo } from "@/lib/types";

const MEMO_CACHE_KEY = "cap.memo.cache.v1";
export const MEMO_CACHE_TTL_MS = 10 * 60 * 1000;

const isRemoteImageUrl = (url: string) =>
  url.startsWith("http://") || url.startsWith("https://");

export const sanitizeMemoForCache = (memo: Memo): Memo => {
  if (memo.id.startsWith("local-")) {
    return {
      ...memo,
      images: [],
      hasImages: false,
      imageCount: 0,
    };
  }
  const remoteImages = (memo.images ?? []).filter(isRemoteImageUrl);
  if (remoteImages.length === (memo.images ?? []).length) {
    return memo;
  }
  return {
    ...memo,
    images: remoteImages,
  };
};

export const sanitizeMemosForCache = (memos: Memo[]): Memo[] =>
  memos.map(sanitizeMemoForCache);

type MemoCachePayload = {
  userId: string;
  updatedAt: string;
  memos: Memo[];
};

export type MemoCacheEntry = MemoCachePayload & {
  isExpired: boolean;
  expiresAtMs: number | null;
};

const isMemoCachePayload = (value: unknown): value is MemoCachePayload => {
  if (!value || typeof value !== "object") return false;
  const record = value as MemoCachePayload;
  return (
    typeof record.userId === "string" &&
    typeof record.updatedAt === "string" &&
    Array.isArray(record.memos)
  );
};

const getExpiresAtMs = (updatedAt: string): number | null => {
  const updatedAtMs = Date.parse(updatedAt);
  if (!Number.isFinite(updatedAtMs)) return null;
  return updatedAtMs + MEMO_CACHE_TTL_MS;
};

export const readMemoCache = (userId: string): MemoCacheEntry | null => {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(MEMO_CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as unknown;
    if (!isMemoCachePayload(parsed)) return null;
    if (parsed.userId !== userId) return null;
    const expiresAtMs = getExpiresAtMs(parsed.updatedAt);
    const isExpired =
      expiresAtMs === null ? true : Date.now() > expiresAtMs;
    return {
      ...parsed,
      isExpired,
      expiresAtMs,
    };
  } catch (error) {
    console.warn("Failed to read memo cache", error);
    return null;
  }
};

export const writeMemoCache = (userId: string, memos: Memo[]): void => {
  if (typeof window === "undefined") return;
  try {
    const payload: MemoCachePayload = {
      userId,
      updatedAt: new Date().toISOString(),
      memos,
    };
    window.localStorage.setItem(MEMO_CACHE_KEY, JSON.stringify(payload));
  } catch (error) {
    console.warn("Failed to write memo cache", error);
  }
};
