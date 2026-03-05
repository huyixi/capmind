import { useCallback, useRef, useState } from "react";
import type { AuthUser as User } from "@supabase/supabase-js";
import type { Memo } from "@/lib/types";
import type { MemoComposerSubmitResult } from "@/components/memo-composer/logic/types";
import type { MemoComposerActions } from "@/components/memo-list/logic/container";
import { useDraftStorage } from "@/hooks/use-draft-storage";
import { uploadMemoImages } from "@/lib/memo-image-upload";

const LOCAL_ID_PREFIX = "local-";

type ComposerPayload = {
  text: string;
  images: File[];
  existingImageUrls: string[];
};

interface UseMemoComposerControllerOptions {
  isOnline: boolean;
  resolveSubmitUser: () => Promise<User | null>;
  draftStorageKey: string;
}

function createLocalId() {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return `${LOCAL_ID_PREFIX}${crypto.randomUUID()}`;
  }
  return `${LOCAL_ID_PREFIX}${Math.random().toString(36).slice(2)}`;
}

export function useMemoComposerController({
  isOnline,
  resolveSubmitUser,
  draftStorageKey,
}: UseMemoComposerControllerOptions) {
  const composerActionsRef = useRef<MemoComposerActions | null>(null);
  const {
    draftText,
    loadDraftText,
    handleDraftTextChange,
    clearDraftText,
  } = useDraftStorage(draftStorageKey);
  const [isComposerOpen, setIsComposerOpen] = useState(false);
  const [composerMode, setComposerMode] = useState<"create" | "edit">("create");
  const [editingMemo, setEditingMemo] = useState<Memo | null>(null);
  const [editingImages, setEditingImages] = useState<string[]>([]);
  const [canEditImages, setCanEditImages] = useState(true);

  const resetComposerState = useCallback(() => {
    setIsComposerOpen(false);
    setComposerMode("create");
    setEditingMemo(null);
    setEditingImages([]);
    setCanEditImages(true);
  }, []);

  const openCreateComposer = useCallback(() => {
    loadDraftText();
    setComposerMode("create");
    setEditingMemo(null);
    setEditingImages([]);
    setCanEditImages(true);
    setIsComposerOpen(true);
  }, [loadDraftText]);

  const resolveEditImages = useCallback(
    async (memo: Memo): Promise<string[] | null> => {
      const memoImages = memo.images ?? [];
      if (!memo.hasImages || memoImages.length > 0) {
        return memoImages;
      }
      if (!isOnline) {
        return null;
      }

      const [{ createClient }, { MEMO_IMAGES_BUCKET, MEMO_IMAGE_URL_TTL_SECONDS }, { createSignedImageUrls }] =
        await Promise.all([
          import("@/lib/supabase/client"),
          import("@/lib/memo-constants"),
          import("@/lib/supabase/storage"),
        ]);
      const supabase = createClient();

      const { data, error } = await supabase
        .from("memo_images")
        .select("url, sort_order")
        .eq("memo_id", memo.id)
        .order("sort_order", { ascending: true });

      if (error) {
        console.error("Error fetching memo images:", error);
        return null;
      }

      const rawImageUrls = (data ?? []).map((row: { url: string }) => row.url);
      if (rawImageUrls.length === 0) return [];

      return await createSignedImageUrls(
        supabase,
        MEMO_IMAGES_BUCKET,
        rawImageUrls,
        MEMO_IMAGE_URL_TTL_SECONDS,
      );
    },
    [isOnline],
  );

  const handleEditOpen = useCallback(
    async (memo: Memo) => {
      setComposerMode("edit");
      setEditingMemo(memo);
      setEditingImages([]);

      let resolvedImages: string[] = memo.images ?? [];
      let nextCanEditImages = isOnline && !memo.id.startsWith(LOCAL_ID_PREFIX);

      if (memo.hasImages && resolvedImages.length === 0) {
        const fetched = await resolveEditImages(memo);
        if (fetched === null) {
          nextCanEditImages = false;
        } else {
          resolvedImages = fetched;
        }
      }

      setEditingImages(resolvedImages);
      setCanEditImages(nextCanEditImages);
      setIsComposerOpen(true);
    },
    [isOnline, resolveEditImages],
  );

  const handleComposerOpenChange = useCallback(
    (open: boolean) => {
      if (open) {
        setIsComposerOpen(true);
        return;
      }
      resetComposerState();
    },
    [resetComposerState],
  );

  const submitEdit = useCallback(
    async (payload: ComposerPayload): Promise<MemoComposerSubmitResult> => {
      if (!editingMemo) {
        return {
          ok: false,
          error: "Unable to update this memo right now.",
          reason: "unknown",
        };
      }

      const actions = composerActionsRef.current;
      if (!actions) {
        return {
          ok: false,
          error: "Editor is still loading. Please try again.",
          reason: "unknown",
        };
      }

      await actions.handleUpdate({
        id: editingMemo.id,
        text: payload.text,
        expectedVersion: editingMemo.version,
        existingImageUrls: canEditImages
          ? payload.existingImageUrls
          : undefined,
        newImages: canEditImages ? payload.images : [],
      });
      return { ok: true };
    },
    [canEditImages, composerActionsRef, editingMemo],
  );

  const submitCreate = useCallback(
    async (payload: ComposerPayload): Promise<MemoComposerSubmitResult> => {
      const actions = composerActionsRef.current;
      if (actions) {
        actions.handleSubmit(payload.text, payload.images);
        return { ok: true };
      }

      const user = await resolveSubmitUser();
      if (!user) {
        return {
          ok: false,
          error: "Sign in to submit memos. Your draft is still here.",
          reason: "auth",
        };
      }

      const trimmedText = payload.text.trim();
      if (!trimmedText && payload.images.length === 0) {
        return { ok: false, reason: "unknown" };
      }

      if (!isOnline) {
        const [{ enqueueCreate }, { registerOptimisticImages }, { readMemoCache, writeMemoCache }] =
          await Promise.all([
            import("@/lib/offline/memo-queue"),
            import("@/lib/offline/optimistic-images"),
            import("@/lib/memo-cache"),
          ]);
        const createdAt = new Date().toISOString();
        const localId = createLocalId();
        const previewUrls = payload.images.map((file) =>
          URL.createObjectURL(file),
        );
        registerOptimisticImages(localId, previewUrls);

        const optimisticMemo: Memo = {
          id: localId,
          clientId: localId,
          user_id: user.id,
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

        void enqueueCreate({
          clientId: localId,
          text: trimmedText,
          files: payload.images,
          createdAt,
          updatedAt: createdAt,
        });

        const cacheRecord = readMemoCache(user.id);
        const cachedMemos = cacheRecord?.memos ?? [];
        const cachedOptimisticMemo: Memo = {
          ...optimisticMemo,
          images: [],
          hasImages: false,
          imageCount: 0,
        };
        const nextCached = [
          cachedOptimisticMemo,
          ...cachedMemos.filter((memo) => memo.id !== localId),
        ];
        writeMemoCache(user.id, nextCached);
        window.dispatchEvent(
          new CustomEvent("memo-offline-created", {
            detail: { memo: optimisticMemo },
          }),
        );
        return { ok: true };
      }

      const [{ createClient }] = await Promise.all([
        import("@/lib/supabase/client"),
      ]);
      const supabase = createClient();

      try {
        const imagePaths =
          payload.images.length > 0
            ? await uploadMemoImages(payload.images)
            : [];

        const response = await fetch("/api/memos", {
          method: "POST",
          credentials: "include",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            text: trimmedText,
            imageUrls: imagePaths,
          }),
        });

        if (response.status === 401 || response.status === 403) {
          return {
            ok: false,
            error: "Sign in to submit memos. Your draft is still here.",
            reason: "auth",
          };
        }

        if (!response.ok) {
          return {
            ok: false,
            error: "Failed to submit. Please try again.",
            reason: "unknown",
          };
        }

        return { ok: true };
      } catch (error) {
        console.error("Error creating memo:", error);
        const message =
          error instanceof Error && error.message.trim().length > 0
            ? error.message
            : "Failed to submit. Please try again.";
        return {
          ok: false,
          error: message,
          reason: message.includes("Sign in") ? "auth" : "unknown",
        };
      }
    },
    [composerActionsRef, isOnline, resolveSubmitUser],
  );

  const handleComposerSubmit = useCallback(
    async (payload: ComposerPayload): Promise<MemoComposerSubmitResult> => {
      if (composerMode === "edit") {
        return submitEdit(payload);
      }
      return submitCreate(payload);
    },
    [composerMode, submitCreate, submitEdit],
  );

  const registerComposerActions = useCallback(
    (actions: MemoComposerActions | null) => {
      composerActionsRef.current = actions;
    },
    [],
  );

  return {
    isComposerOpen,
    composerMode,
    editingMemo,
    editingImages,
    canEditImages,
    draftText,
    handleDraftTextChange,
    clearDraftText,
    openCreateComposer,
    handleEditOpen,
    handleComposerOpenChange,
    handleComposerSubmit,
    resetComposerState,
    registerComposerActions,
  };
}
