"use client";

import { MemoComposerDialog } from "./dialog";
import type { MemoComposerSubmitResult } from "../logic/types";
import type { Memo } from "@/lib/types";

export interface MemoComposerPanelProps {
  open: boolean;
  mode: "create" | "edit";
  editingMemo: Memo | null;
  editingImages: string[];
  canEditImages: boolean;
  draftText: string;
  onOpenChange: (open: boolean) => void;
  onSubmit: (payload: {
    text: string;
    images: File[];
    existingImageUrls: string[];
  }) => Promise<MemoComposerSubmitResult>;
  onDraftTextChange?: (value: string) => void;
  onDraftClear?: () => void;
  onComposerFocus?: () => void;
  onComposerFirstKeystroke?: () => void;
}

export function MemoComposerPanel({
  open,
  mode,
  editingMemo,
  editingImages,
  canEditImages,
  draftText,
  onOpenChange,
  onSubmit,
  onDraftTextChange,
  onDraftClear,
  onComposerFocus,
  onComposerFirstKeystroke,
}: MemoComposerPanelProps) {
  const isEditMode = mode === "edit";

  return (
    <MemoComposerDialog
      open={open}
      onOpenChange={onOpenChange}
      onSubmit={onSubmit}
      mode={mode}
      initialText={isEditMode ? (editingMemo?.text ?? "") : draftText}
      initialImages={isEditMode ? editingImages : []}
      allowImages={isEditMode ? canEditImages : true}
      onDraftTextChange={isEditMode ? undefined : onDraftTextChange}
      onDraftClear={isEditMode ? undefined : onDraftClear}
      onComposerFocus={onComposerFocus}
      onComposerFirstKeystroke={onComposerFirstKeystroke}
      hasFallbackImages={
        isEditMode &&
        !canEditImages &&
        Boolean(editingMemo?.hasImages) &&
        editingImages.length === 0
      }
    />
  );
}
