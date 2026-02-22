import { useCallback, useEffect } from "react";
import type { SupabaseClient, User } from "@supabase/supabase-js";
import type { SWRInfiniteKeyedMutator } from "swr/infinite";
import { Memo } from "@/lib/types";
import {
  MEMO_IMAGE_URL_TTL_SECONDS,
  MEMO_IMAGES_BUCKET,
} from "@/lib/memo-constants";
import {
  createSignedImageUrls,
  extractStoragePath,
} from "@/lib/supabase/storage";
import {
  nextMemoVersion,
  normalizeExpectedVersion,
  normalizeMemoVersion,
} from "@/lib/memo-utils";
import {
  enqueueCreate,
  enqueueDelete,
  enqueueRestore,
  enqueueUpdate,
  removePendingCreate,
  updatePendingCreate,
} from "@/lib/offline/memo-queue";
import {
  cleanupAllOptimisticImages,
  cleanupOptimisticImages as cleanupOptimisticImagesStore,
  registerOptimisticImages,
} from "@/lib/offline/optimistic-images";

type MemoRow = {
  id: string;
  user_id: string;
  text: string;
  created_at: string;
  updated_at: string;
  version: string | number;
  deleted_at: string | null;
  memo_images?: { url: string; sort_order: number }[];
};

type UpdatePayload = {
  id: string;
  text: string;
  expectedVersion: string;
  existingImageUrls?: string[];
  newImages?: File[];
};

interface UseMemoMutationsOptions {
  initialUser: User | null;
  isOnline: boolean;
  mutate: SWRInfiniteKeyedMutator<Memo[][]>;
  resolvePages: (pages: Memo[][] | undefined) => Memo[][];
  supabase: SupabaseClient;
}

export function useMemoMutations({
  initialUser,
  isOnline,
  mutate,
  resolvePages,
  supabase,
}: UseMemoMutationsOptions) {
  const cleanupOptimisticImages = useCallback((clientId: string) => {
    cleanupOptimisticImagesStore(clientId);
  }, []);

  useEffect(() => {
    return () => {
      cleanupAllOptimisticImages();
    };
  }, []);

  const insertMemo = useCallback((pages: Memo[][] | undefined, memo: Memo) => {
    if (!pages || pages.length === 0) {
      return [[memo]];
    }
    return [[memo, ...pages[0]], ...pages.slice(1)];
  }, []);

  const replaceMemo = useCallback(
    (pages: Memo[][] | undefined, id: string, memo: Memo) => {
      if (!pages || pages.length === 0) {
        return [[memo]];
      }
      let replaced = false;
      const nextPages = pages.map((page) =>
        page.map((item) => {
          if (item.id === id) {
            replaced = true;
            const nextClientId =
              memo.clientId ??
              item.clientId ??
              (id.startsWith("local-") ? id : undefined);
            return { ...memo, clientId: nextClientId };
          }
          return item;
        }),
      );
      if (!replaced) {
        return insertMemo(nextPages, memo);
      }
      return nextPages;
    },
    [insertMemo],
  );

  const removeMemo = useCallback((pages: Memo[][] | undefined, id: string) => {
    if (!pages) return [];
    return pages.map((page) => page.filter((memo) => memo.id !== id));
  }, []);

  const updateActiveListCache = useCallback(
    (memo: Memo) => {
      const nextMemo = {
        ...memo,
        serverVersion: normalizeMemoVersion(memo.serverVersion ?? memo.version),
        hasConflict: false,
        conflictServerMemo: undefined,
        conflictType: undefined,
      };
      mutate(
        (current) =>
          replaceMemo(resolvePages(current), nextMemo.id, nextMemo),
        { revalidate: false },
      );
    },
    [mutate, replaceMemo, resolvePages],
  );

  const uploadImages = useCallback(
    async (files: File[], userId: string): Promise<string[]> => {
      const uploads = files.map(async (file) => {
        const fileExt = file.name.split(".").pop() || "bin";
        const uniqueId =
          typeof crypto !== "undefined" && crypto.randomUUID
            ? crypto.randomUUID()
            : Math.random().toString(36).slice(2);
        const fileName = `${userId}/${Date.now()}-${uniqueId}.${fileExt}`;

        const { error: uploadError } = await supabase.storage
          .from(MEMO_IMAGES_BUCKET)
          .upload(fileName, file);

        if (uploadError) {
          console.error("Upload error:", uploadError);
          return null;
        }

        return fileName;
      });

      const uploadedPaths = await Promise.all(uploads);
      return uploadedPaths.filter((path): path is string => Boolean(path));
    },
    [supabase],
  );

  const fetchMemoImagePaths = useCallback(
    async (memoId: string): Promise<string[]> => {
      const { data, error } = await supabase
        .from("memo_images")
        .select("url, sort_order")
        .eq("memo_id", memoId)
        .order("sort_order", { ascending: true });

      if (error) {
        console.error("Error fetching memo image paths:", error);
        return [];
      }

      return (data ?? []).map((row) => row.url);
    },
    [supabase],
  );

  const forkMemoFromConflict = useCallback(
    async (payload: {
      sourceMemoId: string;
      text: string;
    }): Promise<Memo | null> => {
      if (!initialUser) return null;

      const { data: newMemo, error } = await supabase
        .from("memos")
        .insert({
          user_id: initialUser.id,
          text: payload.text,
        })
        .select(
          "id, user_id, text, created_at, updated_at, version, deleted_at",
        )
        .single();

      if (error || !newMemo) {
        console.error("Error creating forked memo:", error);
        return null;
      }

      const rawImageUrls = await fetchMemoImagePaths(payload.sourceMemoId);
      let displayUrls: string[] = [];
      if (rawImageUrls.length > 0) {
        const { error: imageError } = await supabase.from("memo_images").insert(
          rawImageUrls.map((url, index) => ({
            memo_id: newMemo.id,
            url,
            sort_order: index,
          })),
        );

        if (imageError) {
          console.error("Error copying memo images:", imageError);
        } else {
          displayUrls = await createSignedImageUrls(
            supabase,
            MEMO_IMAGES_BUCKET,
            rawImageUrls,
            MEMO_IMAGE_URL_TTL_SECONDS,
          );
        }
      }

      const normalizedVersion = normalizeMemoVersion(newMemo.version);
      return {
        ...newMemo,
        version: normalizedVersion,
        images: displayUrls,
        deleted_at: newMemo.deleted_at ?? null,
        serverVersion: normalizedVersion,
        hasConflict: false,
        conflictServerMemo: undefined,
        conflictType: undefined,
      };
    },
    [fetchMemoImagePaths, initialUser, supabase],
  );

  const fetchServerMemo = useCallback(
    async (memoId: string): Promise<Memo | null> => {
      if (!initialUser) return null;
      const { data, error } = await supabase
        .from("memos")
        .select(
          "id, user_id, text, created_at, updated_at, version, deleted_at, memo_images(url, sort_order)",
        )
        .eq("id", memoId)
        .eq("user_id", initialUser.id)
        .order("sort_order", {
          referencedTable: "memo_images",
          ascending: true,
        })
        .maybeSingle();

      if (error || !data) {
        if (error) {
          console.error("Error fetching server memo:", error);
        }
        return null;
      }

      const memoRow = data as MemoRow;
      const { memo_images, ...rest } = memoRow;
      const rawImageUrls = memo_images?.map((image) => image.url) ?? [];
      const resolvedUrls =
        rawImageUrls.length > 0
          ? await createSignedImageUrls(
              supabase,
              MEMO_IMAGES_BUCKET,
              rawImageUrls,
              MEMO_IMAGE_URL_TTL_SECONDS,
            )
          : [];
      const version = normalizeMemoVersion(rest.version);
      return {
        ...rest,
        version,
        images: resolvedUrls,
        serverVersion: version,
        hasConflict: false,
        conflictServerMemo: undefined,
        conflictType: undefined,
      };
    },
    [initialUser, supabase],
  );

  const shouldShowMemo = useCallback((memo: Memo) => !memo.deleted_at, []);

  const handleSubmit = useCallback(
    (text: string, images: File[]) => {
      if (!initialUser) return;

      const trimmedText = text.trim();
      const createdAt = new Date().toISOString();
      const localId =
        typeof crypto !== "undefined" && crypto.randomUUID
          ? `local-${crypto.randomUUID()}`
          : `local-${Math.random().toString(36).slice(2)}`;
      const previewUrls = images.map((file) => URL.createObjectURL(file));
      registerOptimisticImages(localId, previewUrls);

      const optimisticMemo: Memo = {
        id: localId,
        clientId: localId,
        user_id: initialUser.id,
        text: trimmedText,
        images: previewUrls,
        created_at: createdAt,
        updated_at: createdAt,
        version: "1",
        deleted_at: null,
        serverVersion: "1",
        hasConflict: false,
        conflictServerMemo: undefined,
        conflictType: undefined,
      };

      if (!isOnline) {
        void mutate(
          (current) => insertMemo(resolvePages(current), optimisticMemo),
          { revalidate: false },
        );
        void enqueueCreate({
          clientId: localId,
          text: trimmedText,
          files: images,
          createdAt,
          updatedAt: createdAt,
        });
        return;
      }

      mutate(
        (current) => insertMemo(resolvePages(current), optimisticMemo),
        { revalidate: false },
      );

      void (async () => {
        try {
          let imagePaths: string[] = [];
          if (images.length > 0) {
            imagePaths = await uploadImages(images, initialUser.id);
          }

          const { data: newMemo, error } = await supabase
            .from("memos")
            .insert({
              user_id: initialUser.id,
              text: trimmedText,
            })
            .select(
              "id, user_id, text, created_at, updated_at, version, deleted_at",
            )
            .single();

          if (error) throw error;
          const normalizedNewMemo = {
            ...newMemo,
            version: normalizeMemoVersion(newMemo.version),
          };

          cleanupOptimisticImages(localId);

          let storedImages: string[] = [];
          if (imagePaths.length > 0) {
            const displayUrls = await createSignedImageUrls(
              supabase,
              MEMO_IMAGES_BUCKET,
              imagePaths,
              MEMO_IMAGE_URL_TTL_SECONDS,
            );
            const { error: imageError } = await supabase
              .from("memo_images")
              .insert(
                imagePaths.map((url, index) => ({
                  memo_id: newMemo.id,
                  url,
                  sort_order: index,
                })),
              );
            if (imageError) {
              console.error("Error saving memo images:", imageError);
              storedImages = [];
            } else {
              storedImages = displayUrls;
            }
          }

          mutate(
            (current) =>
              replaceMemo(resolvePages(current), localId, {
                ...normalizedNewMemo,
                images: storedImages,
                serverVersion: normalizedNewMemo.version,
                hasConflict: false,
                conflictServerMemo: undefined,
              }),
            { revalidate: false },
          );
        } catch (error) {
          console.error("Error creating memo:", error);
        }
      })();
    },
    [
      cleanupOptimisticImages,
      initialUser,
      insertMemo,
      isOnline,
      mutate,
      replaceMemo,
      resolvePages,
      uploadImages,
      supabase,
    ],
  );

  const handleUpdate = useCallback(
    async (payload: UpdatePayload) => {
      const updatedAt = new Date().toISOString();
      const trimmedText = payload.text.trim();
      const expectedVersionValue = normalizeExpectedVersion(
        payload.expectedVersion,
      );
      const nextVersion = nextMemoVersion(expectedVersionValue);
      const shouldUpdateImages =
        Array.isArray(payload.existingImageUrls) ||
        (payload.newImages?.length ?? 0) > 0;
      const existingImageUrls = payload.existingImageUrls ?? [];
      const newImageFiles = payload.newImages ?? [];
      const previewUrls = shouldUpdateImages
        ? newImageFiles.map((file) => URL.createObjectURL(file))
        : [];

      if (previewUrls.length > 0) {
        cleanupOptimisticImages(payload.id);
        registerOptimisticImages(payload.id, previewUrls);
      }

      const optimisticImages = shouldUpdateImages
        ? [...existingImageUrls, ...previewUrls]
        : null;

      const applyOptimisticUpdate = (pages: Memo[][] | undefined) => {
        if (!pages) return [];
        return pages.map((page) =>
          page.map((memo) => {
            if (memo.id !== payload.id) return memo;
            const nextMemo: Memo = {
              ...memo,
              text: trimmedText,
              updated_at: updatedAt,
              version: nextVersion,
              hasConflict: false,
              conflictServerMemo: undefined,
              conflictType: undefined,
            };
            if (optimisticImages) {
              nextMemo.images = optimisticImages;
              nextMemo.imageCount = optimisticImages.length;
              nextMemo.hasImages = optimisticImages.length > 0;
            }
            return nextMemo;
          }),
        );
      };

      if (payload.id.startsWith("local-")) {
        mutate(
          (current) => {
            if (!current) return current;
            return current.map((page) =>
              page.map((memo) =>
                memo.id === payload.id
                  ? { ...memo, text: trimmedText, updated_at: updatedAt }
                  : memo,
              ),
            );
          },
          { revalidate: false },
        );
        if (!isOnline) {
          const updated = await updatePendingCreate(payload.id, {
            text: trimmedText,
            updatedAt,
          });
          if (!updated) {
            console.warn("Missing pending create for local memo update.");
          }
        }
        return;
      }

      if (!isOnline) {
        mutate(
          (current) => {
            if (!current) return current;
            return current.map((page) =>
              page.map((memo) =>
                memo.id === payload.id
                  ? { ...memo, text: trimmedText, updated_at: updatedAt }
                  : memo,
              ),
            );
          },
          { revalidate: false },
        );
        await enqueueUpdate({
          memoId: payload.id,
          text: trimmedText,
          updatedAt,
          expectedVersion: expectedVersionValue,
        });
        return;
      }

      void mutate(
        async (current) => {
          let imageUrls: string[] | undefined;
          try {
            if (shouldUpdateImages) {
              const existing = existingImageUrls
                .map(
                  (raw) =>
                    extractStoragePath(raw, MEMO_IMAGES_BUCKET) ?? raw.trim(),
                )
                .filter(Boolean);
              let uploaded: string[] = [];
              if (newImageFiles.length > 0 && initialUser) {
                uploaded = await uploadImages(newImageFiles, initialUser.id);
              }
              imageUrls = [...existing, ...uploaded];
            }

            const response = await fetch(`/api/memos/${payload.id}`, {
              method: "PATCH",
              credentials: "include",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                text: trimmedText,
                expectedVersion: expectedVersionValue,
                imageUrls,
              }),
            });

            if (response.status === 409) {
              const responsePayload = await response.json();
              let serverMemo = responsePayload?.memo as Memo | undefined;
              if (!serverMemo) {
                const fetched = await fetchServerMemo(payload.id);
                serverMemo = fetched ?? undefined;
              }
              const forkedMemo = await forkMemoFromConflict({
                sourceMemoId: payload.id,
                text: trimmedText,
              });
              let nextPages = resolvePages(current);
              if (serverMemo) {
                const normalizedVersion = normalizeMemoVersion(
                  serverMemo.version,
                );
                const nextServerMemo = {
                  ...serverMemo,
                  version: normalizedVersion,
                  serverVersion: normalizedVersion,
                  hasConflict: false,
                  conflictServerMemo: undefined,
                  conflictType: undefined,
                };
                if (shouldShowMemo(nextServerMemo)) {
                  nextPages = replaceMemo(nextPages, payload.id, nextServerMemo);
                } else {
                  nextPages = removeMemo(nextPages, payload.id);
                }
              }
              if (forkedMemo && shouldShowMemo(forkedMemo)) {
                nextPages = insertMemo(nextPages, forkedMemo);
              }
              return nextPages;
            }

            if (!response.ok) {
              throw new Error("Failed to update memo");
            }

            const responsePayload = await response.json();
            const updatedMemo = responsePayload?.memo as Memo | undefined;
            if (!updatedMemo) {
              throw new Error("Missing memo in update response");
            }

            const basePages = resolvePages(current);
            return basePages.map((page) =>
              page.map((memo) =>
                memo.id === payload.id
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
          } finally {
            if (previewUrls.length > 0) {
              cleanupOptimisticImages(payload.id);
            }
          }
        },
        {
          optimisticData: (current) =>
            applyOptimisticUpdate(resolvePages(current)),
          rollbackOnError: true,
          revalidate: false,
        },
      ).catch((error) => {
        console.error("Error updating memo:", error);
        if (previewUrls.length > 0) {
          cleanupOptimisticImages(payload.id);
        }
      });
    },
    [
      cleanupOptimisticImages,
      fetchServerMemo,
      forkMemoFromConflict,
      initialUser,
      insertMemo,
      isOnline,
      mutate,
      removeMemo,
      replaceMemo,
      resolvePages,
      shouldShowMemo,
      uploadImages,
    ],
  );

  const handleDelete = useCallback(
    async (id: string, _images: string[], expectedVersion: string) => {
      const deletedAt = new Date().toISOString();
      const expectedVersionValue = normalizeExpectedVersion(expectedVersion);
      if (id.startsWith("local-")) {
        cleanupOptimisticImages(id);
        mutate((current) => removeMemo(resolvePages(current), id), {
          revalidate: false,
        });
        if (!isOnline) {
          await removePendingCreate(id);
        }
        return;
      }

      if (!isOnline) {
        mutate((current) => removeMemo(resolvePages(current), id), {
          revalidate: false,
        });
        await enqueueDelete({
          memoId: id,
          deletedAt,
          expectedVersion: expectedVersionValue,
        });
        return;
      }

      try {
        let needsRevalidate = false;
        const nextVersion = nextMemoVersion(expectedVersionValue);
        await mutate(
          async (current) => {
            const basePages = resolvePages(current);
            const { data: deletedMemo, error } = await supabase
              .from("memos")
              .update({
                deleted_at: deletedAt,
                updated_at: deletedAt,
                version: nextVersion,
              })
              .eq("id", id)
              .eq("version", expectedVersionValue)
              .select("id")
              .maybeSingle();

            if (error) throw error;
            if (!deletedMemo) {
              const serverMemo = await fetchServerMemo(id);
              if (serverMemo) {
                if (shouldShowMemo(serverMemo)) {
                  return replaceMemo(basePages, id, serverMemo);
                }
                return removeMemo(basePages, id);
              }
              needsRevalidate = true;
              return removeMemo(basePages, id);
            }

            return removeMemo(basePages, id);
          },
          {
            optimisticData: (current) =>
              removeMemo(resolvePages(current), id),
            rollbackOnError: true,
            revalidate: false,
          },
        );

        if (needsRevalidate) {
          await mutate();
        }
      } catch (error) {
        console.error("Error deleting memo:", error);
        await mutate();
      }
    },
    [
      cleanupOptimisticImages,
      fetchServerMemo,
      isOnline,
      mutate,
      removeMemo,
      replaceMemo,
      resolvePages,
      shouldShowMemo,
      supabase,
    ],
  );

  const handleRestore = useCallback(
    async (memo: Memo): Promise<boolean> => {
      const id = memo.id;
      const restoredAt = new Date().toISOString();
      const expectedVersionValue = normalizeExpectedVersion(memo.version);
      const nextVersion = nextMemoVersion(expectedVersionValue);
      const optimisticMemo: Memo = {
        ...memo,
        deleted_at: null,
        updated_at: restoredAt,
        version: nextVersion,
        serverVersion: nextVersion,
        hasConflict: false,
        conflictServerMemo: undefined,
        conflictType: undefined,
      };
      if (!isOnline) {
        mutate(
          (current) =>
            replaceMemo(resolvePages(current), id, optimisticMemo),
          { revalidate: false },
        );
        await enqueueRestore({
          memoId: id,
          restoredAt,
          expectedVersion: expectedVersionValue,
        });
        return true;
      }

      try {
        let restoredMemoRow: MemoRow | null = null;
        await mutate(
          async (current) => {
            const basePages = resolvePages(current);
            const { data: restoredMemo, error } = await supabase
              .from("memos")
              .update({
                deleted_at: null,
                updated_at: restoredAt,
                version: nextVersion,
              })
              .eq("id", id)
              .eq("version", expectedVersionValue)
              .select(
                "id, user_id, text, created_at, updated_at, version, deleted_at",
              )
              .maybeSingle();

            if (error) throw error;
            if (!restoredMemo) {
              const serverMemo = await fetchServerMemo(id);
              if (serverMemo) {
                if (shouldShowMemo(serverMemo)) {
                  return replaceMemo(basePages, id, serverMemo);
                }
                throw new Error("Restore rejected by server.");
              }
              throw new Error("Restore failed without server memo.");
            }

            restoredMemoRow = restoredMemo as MemoRow;
            const normalizedVersion = normalizeMemoVersion(
              restoredMemoRow.version,
            );
            return replaceMemo(basePages, id, {
              ...optimisticMemo,
              ...restoredMemoRow,
              version: normalizedVersion,
              serverVersion: normalizedVersion,
              deleted_at: restoredMemoRow.deleted_at ?? null,
            });
          },
          {
            optimisticData: (current) =>
              replaceMemo(resolvePages(current), id, optimisticMemo),
            rollbackOnError: true,
            revalidate: false,
          },
        );

        if (restoredMemoRow) {
          const restoredMemo = restoredMemoRow as MemoRow;
          const normalizedRestoredMemo = {
            ...restoredMemo,
            version: normalizeMemoVersion(restoredMemo.version),
          };
          const { data: imageRows, error: imageError } = await supabase
            .from("memo_images")
            .select("url, sort_order")
            .eq("memo_id", id)
            .order("sort_order", { ascending: true });

          if (imageError) {
            console.error("Error fetching memo images:", imageError);
          }

          const rawImageUrls = imageRows?.map((row) => row.url) ?? [];
          const resolvedUrls =
            rawImageUrls.length > 0
              ? await createSignedImageUrls(
                  supabase,
                  MEMO_IMAGES_BUCKET,
                  rawImageUrls,
                  MEMO_IMAGE_URL_TTL_SECONDS,
                )
              : [];

          const hydratedMemo = {
            ...normalizedRestoredMemo,
            images: resolvedUrls,
            hasImages:
              rawImageUrls.length > 0 ? true : optimisticMemo.hasImages,
            imageCount:
              rawImageUrls.length > 0
                ? rawImageUrls.length
                : optimisticMemo.imageCount,
          };
          updateActiveListCache(hydratedMemo);
        }
        return true;
      } catch (error) {
        console.error("Error restoring memo:", error);
        await mutate();
        return false;
      }
    },
    [
      fetchServerMemo,
      isOnline,
      mutate,
      replaceMemo,
      resolvePages,
      shouldShowMemo,
      supabase,
      updateActiveListCache,
    ],
  );

  return {
    cleanupOptimisticImages,
    fetchServerMemo,
    forkMemoFromConflict,
    handleDelete,
    handleRestore,
    handleSubmit,
    handleUpdate,
    insertMemo,
    removeMemo,
    replaceMemo,
    uploadImages,
  };
}
