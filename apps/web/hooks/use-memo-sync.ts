import { useCallback, useEffect, useRef, useState } from "react";
import type { SupabaseClient, User } from "@supabase/supabase-js";
import type { SWRInfiniteKeyedMutator } from "swr/infinite";
import { Memo } from "@/lib/types";
import { normalizeExpectedVersion, normalizeMemoVersion } from "@/lib/memo-utils";
import {
  getOutboxItems,
  removeOutboxItem,
} from "@/lib/offline/memo-queue";

type FlushResult = {
  didSync: boolean;
  hadError: boolean;
};

type ReplaceMemo = (
  pages: Memo[][] | undefined,
  id: string,
  memo: Memo,
) => Memo[][];
type RemoveMemo = (
  pages: Memo[][] | undefined,
  id: string,
) => Memo[][];

interface UseMemoSyncOptions {
  initialUser: User | null;
  isOnline: boolean;
  autoSyncEnabled?: boolean;
  mutate: SWRInfiniteKeyedMutator<Memo[][]>;
  resolvePages: (pages: Memo[][] | undefined) => Memo[][];
  replaceMemo: ReplaceMemo;
  removeMemo: RemoveMemo;
  fetchServerMemo: (memoId: string) => Promise<Memo | null>;
  uploadImages: (files: File[], userId: string) => Promise<string[]>;
  supabase: SupabaseClient;
  cleanupOptimisticImages?: (clientId: string) => void;
}

export function useMemoSync({
  initialUser,
  isOnline,
  autoSyncEnabled = true,
  mutate,
  resolvePages,
  replaceMemo,
  removeMemo,
  fetchServerMemo,
  uploadImages,
  supabase,
  cleanupOptimisticImages,
}: UseMemoSyncOptions) {
  const syncPromiseRef = useRef<Promise<FlushResult> | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);

  const shouldShowMemo = useCallback((memo: Memo) => !memo.deleted_at, []);

  const normalizeServerMemo = useCallback((memo: Memo) => {
    const normalizedVersion = normalizeMemoVersion(memo.version);
    return {
      ...memo,
      version: normalizedVersion,
      serverVersion: normalizedVersion,
      hasConflict: false,
      conflictServerMemo: undefined,
      conflictType: undefined,
    };
  }, []);

  const flushOutbox = useCallback(async (): Promise<FlushResult> => {
    if (!initialUser || !navigator.onLine) {
      return { didSync: false, hadError: false };
    }
    if (syncPromiseRef.current) {
      return syncPromiseRef.current;
    }

    setIsSyncing(true);
    const run = (async (): Promise<FlushResult> => {
      let didSync = false;
      let hadError = false;

      try {
        const items = await getOutboxItems();
        for (const item of items) {
          if (item.type === "create") {
            let imagePaths: string[] = [];
            if (item.files.length > 0) {
              imagePaths = await uploadImages(item.files, initialUser.id);
            }

            const response = await fetch("/api/memos", {
              method: "POST",
              credentials: "include",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                text: item.text,
                imageUrls: imagePaths,
              }),
            });

            if (!response.ok) {
              throw new Error("Failed to create memo");
            }

            const payload = await response.json();
            const createdMemo = payload?.memo as Memo | undefined;
            if (!createdMemo) {
              throw new Error("Missing memo in create response");
            }

            await mutate(
              (current) =>
                replaceMemo(resolvePages(current), item.clientId, {
                  ...createdMemo,
                  version: normalizeMemoVersion(createdMemo.version),
                  serverVersion: normalizeMemoVersion(createdMemo.version),
                  hasConflict: false,
                  conflictServerMemo: undefined,
                  conflictType: undefined,
                }),
              { revalidate: false },
            );

            cleanupOptimisticImages?.(item.clientId);
            if (item.id !== undefined) {
              await removeOutboxItem(item.id);
            }
            didSync = true;
            continue;
          }

          if (item.type === "update") {
            const expectedVersion = normalizeExpectedVersion(
              item.expectedVersion,
            );
            const response = await fetch(`/api/memos/${item.memoId}`, {
              method: "PATCH",
              credentials: "include",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                text: item.text,
                expectedVersion,
              }),
            });

            if (response.status === 409) {
              const payload = await response.json();
              let serverMemo = payload?.memo as Memo | undefined;
              const forkedMemo = payload?.forkedMemo as Memo | undefined;
              if (!serverMemo) {
                const fetched = await fetchServerMemo(item.memoId);
                serverMemo = fetched ?? undefined;
              }
              mutate(
                (current) => {
                  if (!current) return current;
                  let nextPages = resolvePages(current);
                  if (serverMemo) {
                    const nextServerMemo = normalizeServerMemo(serverMemo);
                    if (shouldShowMemo(nextServerMemo)) {
                      nextPages = replaceMemo(
                        nextPages,
                        item.memoId,
                        nextServerMemo,
                      );
                    } else {
                      nextPages = removeMemo(nextPages, item.memoId);
                    }
                  }
                  if (forkedMemo) {
                    const normalizedForkedVersion = normalizeMemoVersion(
                      forkedMemo.version,
                    );
                    const nextForkedMemo = {
                      ...forkedMemo,
                      version: normalizedForkedVersion,
                      serverVersion: normalizedForkedVersion,
                      hasConflict: false,
                      conflictServerMemo: undefined,
                      conflictType: undefined,
                    };
                    if (shouldShowMemo(nextForkedMemo)) {
                      if (!nextPages || nextPages.length === 0) {
                        nextPages = [[nextForkedMemo]];
                      } else {
                        nextPages = [
                          [nextForkedMemo, ...nextPages[0]],
                          ...nextPages.slice(1),
                        ];
                      }
                    }
                  }
                  if (!serverMemo && !forkedMemo) {
                    throw new Error("Conflict response missing memo payload");
                  }
                  return nextPages;
                },
                { revalidate: false },
              );
              if (item.id !== undefined) {
                await removeOutboxItem(item.id);
              }
              didSync = true;
              continue;
            }

            if (!response.ok) {
              throw new Error("Failed to update memo");
            }

            const payload = await response.json();
            const updatedMemo = payload?.memo as Memo | undefined;
            if (!updatedMemo) {
              throw new Error("Missing memo in update response");
            }
            mutate(
              (current) => {
                if (!current) return current;
                return current.map((page) =>
                  page.map((memo) =>
                    memo.id === item.memoId
                      ? {
                          ...memo,
                          ...updatedMemo,
                          clientId: memo.clientId ?? updatedMemo.clientId,
                          serverVersion: updatedMemo.version,
                          hasConflict: false,
                          conflictServerMemo: undefined,
                          conflictType: undefined,
                        }
                      : memo,
                  ),
                );
              },
              { revalidate: false },
            );
            if (item.id !== undefined) {
              await removeOutboxItem(item.id);
            }
            didSync = true;
            continue;
          }

          if (item.type === "delete") {
            const expectedVersion = normalizeExpectedVersion(
              item.expectedVersion,
            );
            const response = await fetch(`/api/memos/${item.memoId}`, {
              method: "DELETE",
              credentials: "include",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                expectedVersion,
                deletedAt: item.deletedAt,
              }),
            });

            if (response.status === 401 || response.status === 403) {
              break;
            }

            if (response.status === 409) {
              if (item.id !== undefined) {
                await removeOutboxItem(item.id);
              }
              const serverMemo = await fetchServerMemo(item.memoId);
              if (serverMemo) {
                const nextServerMemo = normalizeServerMemo(serverMemo);
                if (shouldShowMemo(nextServerMemo)) {
                  mutate(
                    (current) =>
                      replaceMemo(
                        resolvePages(current),
                        item.memoId,
                        nextServerMemo,
                      ),
                    { revalidate: false },
                  );
                } else {
                  mutate(
                    (current) => removeMemo(resolvePages(current), item.memoId),
                    { revalidate: false },
                  );
                }
              }
              didSync = true;
              continue;
            }

            if (!response.ok) {
              throw new Error("Failed to delete memo");
            }

            if (item.id !== undefined) {
              await removeOutboxItem(item.id);
            }
            didSync = true;
            continue;
          }

          if (item.type === "restore") {
            const expectedVersion = normalizeExpectedVersion(
              item.expectedVersion,
            );
            const response = await fetch(`/api/memos/${item.memoId}`, {
              method: "PATCH",
              credentials: "include",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                action: "restore",
                expectedVersion,
                restoredAt: item.restoredAt,
              }),
            });

            if (response.status === 401 || response.status === 403) {
              break;
            }

            if (response.status === 409) {
              if (item.id !== undefined) {
                await removeOutboxItem(item.id);
              }
              const serverMemo = await fetchServerMemo(item.memoId);
              if (serverMemo) {
                const nextServerMemo = normalizeServerMemo(serverMemo);
                if (shouldShowMemo(nextServerMemo)) {
                  mutate(
                    (current) =>
                      replaceMemo(
                        resolvePages(current),
                        item.memoId,
                        nextServerMemo,
                      ),
                    { revalidate: false },
                  );
                } else {
                  mutate(
                    (current) => removeMemo(resolvePages(current), item.memoId),
                    { revalidate: false },
                  );
                }
              }
              didSync = true;
              continue;
            }

            if (!response.ok) {
              throw new Error("Failed to restore memo");
            }

            const payload = await response.json();
            const restoredMemo = payload?.memo as Memo | undefined;
            if (restoredMemo) {
              mutate(
                (current) =>
                  replaceMemo(
                    resolvePages(current),
                    item.memoId,
                    normalizeServerMemo(restoredMemo),
                  ),
                { revalidate: false },
              );
            }

            if (item.id !== undefined) {
              await removeOutboxItem(item.id);
            }
            didSync = true;
          }
        }
      } catch (error) {
        console.error("Error syncing offline queue:", error);
        hadError = true;
      }

      return { didSync, hadError };
    })();

    syncPromiseRef.current = run;

    try {
      return await run;
    } finally {
      syncPromiseRef.current = null;
      setIsSyncing(false);
    }
  }, [
    fetchServerMemo,
    initialUser,
    mutate,
    removeMemo,
    replaceMemo,
    resolvePages,
    shouldShowMemo,
    normalizeServerMemo,
    supabase,
    uploadImages,
    cleanupOptimisticImages,
    setIsSyncing,
  ]);

  useEffect(() => {
    if (!autoSyncEnabled) return;
    if (!initialUser || !isOnline) return;
    void (async () => {
      const result = await flushOutbox();
      if (result.didSync && !result.hadError) {
        await mutate();
      }
    })();
  }, [autoSyncEnabled, flushOutbox, initialUser, isOnline, mutate]);

  return { flushOutbox, isSyncing };
}
