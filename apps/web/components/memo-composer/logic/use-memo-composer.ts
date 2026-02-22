"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent, KeyboardEvent } from "react";
import type { MemoComposerProps } from "./types";

const DEFAULT_MAX_IMAGES = 9;

export type MemoComposerImageItem = {
  kind: "existing" | "new";
  url: string;
  index: number;
};

export function useMemoComposer({
  onSubmit,
  maxImages = DEFAULT_MAX_IMAGES,
  open,
  onOpenChange,
  mode = "create",
  initialText = "",
  allowImages,
  initialImages = [],
  hasFallbackImages = false,
  submitLabel,
  placeholder,
  title,
  onDraftTextChange,
  onDraftClear,
}: MemoComposerProps) {
  const [text, setText] = useState(initialText);
  const [images, setImages] = useState<File[]>([]);
  const [previews, setPreviews] = useState<string[]>([]);
  const [existingImages, setExistingImages] = useState<string[]>(initialImages);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [canExpand, setCanExpand] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const createDraftImagesRef = useRef<File[]>([]);
  const imagesRef = useRef<File[]>([]);
  const previewsRef = useRef<string[]>([]);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const wasOpenRef = useRef(open);
  const lastModeRef = useRef(mode);
  const canManageImages = allowImages ?? true;
  const resolvedSubmitLabel = useMemo(
    () => submitLabel ?? (mode === "edit" ? "Save" : "Submit"),
    [mode, submitLabel],
  );
  const resolvedPlaceholder = useMemo(
    () =>
      placeholder ??
      (mode === "edit" ? "Edit your memo..." : "What's on your mind?"),
    [mode, placeholder],
  );
  const resolvedTitle = useMemo(
    () => title ?? (mode === "edit" ? "编辑 Memo" : "新建 Memo"),
    [mode, title],
  );
  const trimmedText = useMemo(() => text.trim(), [text]);
  const initialTrimmed = useMemo(() => initialText.trim(), [initialText]);
  const areImagesEqual = useMemo(
    () =>
      existingImages.length === initialImages.length &&
      existingImages.every((value, index) => value === initialImages[index]),
    [existingImages, initialImages],
  );
  const hasImageChanges = useMemo(
    () => !areImagesEqual || images.length > 0,
    [areImagesEqual, images.length],
  );
  const hasContent = useMemo(
    () =>
      trimmedText.length > 0 ||
      existingImages.length > 0 ||
      images.length > 0 ||
      (mode === "edit" && hasFallbackImages),
    [
      existingImages.length,
      hasFallbackImages,
      images.length,
      mode,
      trimmedText.length,
    ],
  );
  const isUnchanged = useMemo(
    () => mode === "edit" && trimmedText === initialTrimmed && !hasImageChanges,
    [hasImageChanges, initialTrimmed, mode, trimmedText],
  );
  const canSubmit = useMemo(() => hasContent && !isUnchanged, [
    hasContent,
    isUnchanged,
  ]);

  const resetFileInput = useCallback(() => {
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  }, []);

  const clearPreviews = useCallback(() => {
    setPreviews((current) => {
      current.forEach((url) => URL.revokeObjectURL(url));
      return [];
    });
  }, []);

  const resetComposer = useCallback(() => {
    setText("");
    setImages([]);
    setExistingImages([]);
    clearPreviews();
    resetFileInput();
    setIsFullscreen(false);
    setSubmitError(null);
    if (mode === "create") {
      createDraftImagesRef.current = [];
      onDraftClear?.();
    }
  }, [clearPreviews, mode, onDraftClear, resetFileInput]);

  const handleAddImageClick = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleSubmit = useCallback(async () => {
    if (!canSubmit || isSubmitting) return;
    setIsSubmitting(true);
    setSubmitError(null);
    try {
      const result = await onSubmit({
        text: trimmedText,
        images: canManageImages ? images : [],
        existingImageUrls: existingImages,
      });
      if (result && "ok" in result && result.ok === false) {
        setSubmitError(result.error ?? "Unable to submit. Please try again.");
        return;
      }
      resetComposer();
      onOpenChange(false);
    } catch (error) {
      console.error("Error submitting memo:", error);
      setSubmitError("Unable to submit. Please try again.");
    } finally {
      setIsSubmitting(false);
    }
  }, [
    canSubmit,
    canManageImages,
    existingImages,
    images,
    isSubmitting,
    onOpenChange,
    onSubmit,
    resetComposer,
    trimmedText,
  ]);

  const evaluateOverflow = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) {
      setCanExpand((current) => (current ? false : current));
      return;
    }
    const nextValue = textarea.scrollHeight > textarea.clientHeight + 1;
    setCanExpand((current) => (current === nextValue ? current : nextValue));
  }, []);

  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key !== "Enter") return;
      if (event.nativeEvent.isComposing) return;
      if (event.metaKey || event.ctrlKey) {
        event.preventDefault();
        handleSubmit();
        return;
      }

      const textarea = textareaRef.current;
      if (!textarea) return;

      const value = textarea.value;
      const selectionStart = textarea.selectionStart ?? 0;
      const selectionEnd = textarea.selectionEnd ?? 0;
      const lineStart = value.lastIndexOf("\n", selectionStart - 1) + 1;
      const lineToCursor = value.slice(lineStart, selectionStart);
      const unorderedMatch = lineToCursor.match(/^(\s*)-\s*/);
      const orderedMatch = lineToCursor.match(/^(\s*)(\d+)\.\s*/);

      if (!unorderedMatch && !orderedMatch) return;

      event.preventDefault();
      const indent = unorderedMatch?.[1] ?? orderedMatch?.[1] ?? "";
      const prefix = unorderedMatch
        ? `${indent}- `
        : `${indent}${Number(orderedMatch?.[2] ?? 0) + 1}. `;
      const before = value.slice(0, selectionStart);
      const after = value.slice(selectionEnd);
      const nextValue = `${before}\n${prefix}${after}`;

      setText(nextValue);
      setSubmitError(null);
      onDraftTextChange?.(nextValue);

      requestAnimationFrame(() => {
        const cursor = (before + "\n" + prefix).length;
        textarea.focus();
        textarea.setSelectionRange(cursor, cursor);
      });
    },
    [handleSubmit, onDraftTextChange],
  );

  const handleTextChange = useCallback(
    (event: ChangeEvent<HTMLTextAreaElement>) => {
      const nextValue = event.target.value;
      setText(nextValue);
      setSubmitError(null);
      onDraftTextChange?.(nextValue);
    },
    [onDraftTextChange],
  );

  const handleCancel = useCallback(() => {
    onOpenChange(false);
    setIsFullscreen(false);
    if (mode === "edit") {
      resetComposer();
    }
  }, [mode, onOpenChange, resetComposer]);

  const handleOpenChange = useCallback(
    (nextOpen: boolean) => {
      onOpenChange(nextOpen);
      if (!nextOpen) {
        if (mode === "edit") {
          resetComposer();
        }
        setIsFullscreen(false);
      }
    },
    [mode, onOpenChange, resetComposer],
  );

  const handleImageSelect = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      if (!canManageImages) return;
      const fileList = Array.from(event.target.files || []);
      if (fileList.length === 0) return;
      const imageFiles = fileList.filter((file) =>
        file.type.startsWith("image/"),
      );
      if (imageFiles.length === 0) return;
      setSubmitError(null);

      setImages((current) => {
        const remainingSlots = Math.max(
          0,
          maxImages - (current.length + existingImages.length),
        );
        if (remainingSlots === 0) return current;
        const selectedFiles = imageFiles.slice(0, remainingSlots);
        if (selectedFiles.length === 0) return current;
        setPreviews((currentPreviews) => [
          ...currentPreviews,
          ...selectedFiles.map((file) => URL.createObjectURL(file)),
        ]);
        return [...current, ...selectedFiles];
      });

      resetFileInput();
    },
    [canManageImages, existingImages.length, maxImages, resetFileInput],
  );

  const handleRemoveImage = useCallback((index: number) => {
    setSubmitError(null);
    setImages((current) => current.filter((_, i) => i !== index));
    setPreviews((current) => {
      const previewToRevoke = current[index];
      if (previewToRevoke) {
        URL.revokeObjectURL(previewToRevoke);
      }
      return current.filter((_, i) => i !== index);
    });
  }, []);

  const handleRemoveExistingImage = useCallback(
    (index: number) => {
      if (!canManageImages) return;
      setSubmitError(null);
      setExistingImages((current) => current.filter((_, i) => i !== index));
    },
    [canManageImages],
  );

  const focusTextareaToEnd = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return false;
    const length = textarea.value.length;
    textarea.focus({ preventScroll: true });
    textarea.setSelectionRange(length, length);
    return document.activeElement === textarea;
  }, []);

  const handleOpenAutoFocus = useCallback(
    (event: Event) => {
      event.preventDefault();
      const didFocus = focusTextareaToEnd();
      if (!didFocus) {
        requestAnimationFrame(() => {
          focusTextareaToEnd();
        });
      }
    },
    [focusTextareaToEnd],
  );

  const toggleFullscreen = useCallback(() => {
    setIsFullscreen((current) => !current);
  }, []);

  useEffect(() => {
    imagesRef.current = images;
  }, [images]);

  useEffect(() => {
    if (mode !== "create" || !open) return;
    createDraftImagesRef.current = images;
  }, [images, mode, open]);

  useEffect(() => {
    const justOpened = open && !wasOpenRef.current;
    const modeChanged = open && lastModeRef.current !== mode;
    if (!open) {
      wasOpenRef.current = open;
      lastModeRef.current = mode;
      return;
    }
    if (justOpened || modeChanged) {
      setText(initialText);
      setExistingImages(initialImages);
      setSubmitError(null);
      if (mode === "edit") {
        setImages([]);
        clearPreviews();
        resetFileInput();
      }
      if (
        mode === "create" &&
        imagesRef.current.length === 0 &&
        previewsRef.current.length === 0 &&
        createDraftImagesRef.current.length > 0
      ) {
        const restoredImages = createDraftImagesRef.current;
        setImages(restoredImages);
        setPreviews(restoredImages.map((file) => URL.createObjectURL(file)));
        resetFileInput();
      }
      if (initialText) {
        onDraftTextChange?.(initialText);
      }
      if (!canManageImages) {
        setImages([]);
        clearPreviews();
        resetFileInput();
      }
      requestAnimationFrame(focusTextareaToEnd);
    }
    wasOpenRef.current = open;
    lastModeRef.current = mode;
  }, [
    canManageImages,
    clearPreviews,
    focusTextareaToEnd,
    initialImages,
    initialText,
    mode,
    onDraftTextChange,
    open,
    resetFileInput,
  ]);

  useEffect(() => {
    if (open) return;
    setIsFullscreen(false);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const rafId = requestAnimationFrame(evaluateOverflow);
    return () => cancelAnimationFrame(rafId);
  }, [
    evaluateOverflow,
    existingImages.length,
    images.length,
    isFullscreen,
    open,
    previews.length,
    text,
  ]);

  useEffect(() => {
    if (!open) return;
    const handleResize = () => evaluateOverflow();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [evaluateOverflow, open]);

  useEffect(() => {
    previewsRef.current = previews;
  }, [previews]);

  useEffect(() => {
    return () => {
      previewsRef.current.forEach((url) => URL.revokeObjectURL(url));
      previewsRef.current = [];
    };
  }, []);

  const imageItems = useMemo<MemoComposerImageItem[]>(
    () => [
      ...existingImages.map((url, index) => ({
        kind: "existing" as const,
        url,
        index,
      })),
      ...previews.map((url, index) => ({
        kind: "new" as const,
        url,
        index,
      })),
    ],
    [existingImages, previews],
  );
  const totalImages = useMemo(
    () => existingImages.length + images.length,
    [existingImages.length, images.length],
  );

  return {
    open,
    maxImages,
    text,
    textareaRef,
    fileInputRef,
    isFullscreen,
    canExpand,
    canManageImages,
    resolvedTitle,
    resolvedPlaceholder,
    resolvedSubmitLabel,
    submitError,
    isSubmitting,
    canSubmit,
    totalImages,
    imageItems,
    handleAddImageClick,
    handleImageSelect,
    handleRemoveImage,
    handleRemoveExistingImage,
    handleSubmit,
    handleCancel,
    handleOpenChange,
    handleKeyDown,
    handleTextChange,
    handleOpenAutoFocus,
    toggleFullscreen,
  };
}
