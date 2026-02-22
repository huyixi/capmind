"use client";

import {
  memo as reactMemo,
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import dynamic from "next/dynamic";
import { formatTimestampLocal, formatTimestampUtc } from "@/lib/memo-utils";
import { MemoCardActionsSlot, preloadMemoCardActions } from "./actions-slot";
import { MemoCardContent } from "./content";
import { CLAMP_LINES } from "../logic/constants";
import { type MemoCardProps } from "../logic/types";
import { areImagesEqual } from "../logic/image-utils";
import { useMemoImages } from "../logic/use-memo-images";
import {
  copyTextToClipboard,
  getSupportedClipboardImageMimeTypes,
  linkifyMemoText,
  prepareClipboardImageBlobs,
} from "./utils";

const MemoImagePreview = dynamic(
  () => import("./image-preview").then((mod) => mod.MemoImagePreview),
  { ssr: false },
);

const preloadMemoImagePreview = () => {
  void import("./image-preview");
};

const EMPTY_IMAGES: string[] = [];

function MemoCardBase({
  memo,
  onDelete,
  onEdit,
  onRestore,
  isTrash = false,
  isOnline,
}: MemoCardProps) {
  const [isDeleting, setIsDeleting] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const [expandedImageIndex, setExpandedImageIndex] = useState<number | null>(
    null,
  );
  const [shouldShowToggle, setShouldShowToggle] = useState(false);
  const [actionsLoaded, setActionsLoaded] = useState(false);
  const [actionsOpen, setActionsOpen] = useState(false);
  const contentId = useId();
  const memoImages = memo.images ?? EMPTY_IMAGES;
  const memoImageCount = memo.imageCount ?? (memo.hasImages ? 1 : 0);
  const rawMemoText = memo.text;
  const memoText = rawMemoText.trim();
  const contentRef = useRef<HTMLParagraphElement | null>(null);
  const copyResetTimeoutRef = useRef<number | null>(null);
  const hasCopyContent =
    memoText.length > 0 || memoImages.length > 0 || memoImageCount > 0;
  const displayTimestamp =
    isTrash && memo.deleted_at ? memo.deleted_at : memo.created_at;
  const isDeletedTimestamp = isTrash && Boolean(memo.deleted_at);
  const isPending = memo.id.startsWith("local-") && !isOnline;
  const [displayTimestampLabel, setDisplayTimestampLabel] = useState(() => {
    const base = formatTimestampUtc(displayTimestamp);
    return isDeletedTimestamp ? `${base} - deleted` : base;
  });

  const {
    displayImages,
    needsImageFetch,
    isResolvingImages,
    imageAreaRef,
    resolvePendingImages,
    getSignedImageUrls,
  } = useMemoImages({
    memo,
    memoImages,
    memoImageCount,
  });

  const linkifiedMemoText = useMemo(() => {
    if (!rawMemoText) return rawMemoText;
    if (!rawMemoText.includes("http")) return rawMemoText;
    return linkifyMemoText(rawMemoText);
  }, [rawMemoText]);

  const handleActionsHover = useCallback(() => {
    if (actionsLoaded) return;
    preloadMemoCardActions();
    setActionsLoaded(true);
  }, [actionsLoaded]);

  const handleActionsClick = useCallback(() => {
    if (!actionsLoaded) {
      preloadMemoCardActions();
      setActionsLoaded(true);
    }
    setActionsOpen(true);
  }, [actionsLoaded]);

  useEffect(() => {
    const base = formatTimestampLocal(displayTimestamp);
    setDisplayTimestampLabel(isDeletedTimestamp ? `${base} - deleted` : base);
  }, [displayTimestamp, isDeletedTimestamp]);

  const updateShowToggle = useCallback(() => {
    const element = contentRef.current;
    if (!element || memoText.length === 0) {
      setShouldShowToggle(false);
      return;
    }

    if (element.scrollHeight > element.clientHeight + 1) {
      setShouldShowToggle(true);
      return;
    }

    const styles = window.getComputedStyle(element);
    let lineHeight = Number.parseFloat(styles.lineHeight);
    if (!Number.isFinite(lineHeight)) {
      const fontSize = Number.parseFloat(styles.fontSize);
      lineHeight = Number.isFinite(fontSize) ? fontSize * 1.5 : 0;
    }
    if (!lineHeight) {
      setShouldShowToggle(false);
      return;
    }

    const clone = element.cloneNode(true) as HTMLParagraphElement;
    clone.removeAttribute("id");
    clone.style.setProperty("position", "absolute");
    clone.style.setProperty("visibility", "hidden");
    clone.style.setProperty("pointer-events", "none");
    clone.style.setProperty("z-index", "-1");
    clone.style.setProperty("height", "auto");
    clone.style.setProperty("max-height", "none");
    clone.style.setProperty("overflow", "visible");
    clone.style.setProperty("display", "block");
    clone.style.setProperty("-webkit-box-orient", "initial");
    clone.style.setProperty("-webkit-line-clamp", "unset");
    clone.style.setProperty("width", `${element.clientWidth}px`);

    const mountPoint = element.parentElement ?? document.body;
    mountPoint.appendChild(clone);
    const expandedHeight = clone.scrollHeight;
    clone.remove();

    const lineCount = Math.ceil(expandedHeight / lineHeight - 0.01);
    setShouldShowToggle(lineCount > CLAMP_LINES);
  }, [memoText]);

  useLayoutEffect(() => {
    updateShowToggle();
    const element = contentRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => {
      updateShowToggle();
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [updateShowToggle]);

  useEffect(() => {
    return () => {
      if (copyResetTimeoutRef.current) {
        window.clearTimeout(copyResetTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    if (expandedImageIndex === null) return;
    if (displayImages.length === 0) {
      setExpandedImageIndex(null);
      return;
    }
    if (expandedImageIndex >= displayImages.length) {
      setExpandedImageIndex(displayImages.length - 1);
    }
  }, [displayImages.length, expandedImageIndex]);

  const handleDelete = async () => {
    setIsDeleting(true);
    try {
      await onDelete(memo.id, memoImages, memo.version);
    } finally {
      setIsDeleting(false);
    }
  };

  const handleRestore = async () => {
    if (!onRestore) return;
    setIsRestoring(true);
    try {
      await onRestore(memo);
    } finally {
      setIsRestoring(false);
    }
  };

  const handleEdit = () => {
    onEdit?.(memo);
  };

  const handleCardDoubleClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (isTrash || !onEdit || isDeleting || isRestoring) return;
      const target = event.target as HTMLElement | null;
      if (
        target?.closest(
          "button, a, input, textarea, select, [role='button'], [role='link']",
        )
      ) {
        return;
      }
      onEdit(memo);
    },
    [isTrash, onEdit, isDeleting, isRestoring, memo],
  );

  const resolveCopyImageUrls = useCallback(async (): Promise<string[]> => {
    if (displayImages.length > 0) return displayImages;
    if (memoImages.length > 0) return memoImages;
    if (memoImageCount <= 0) return [];
    try {
      return await getSignedImageUrls();
    } catch (error) {
      console.error("Error resolving memo images for copy:", error);
      return [];
    }
  }, [displayImages, getSignedImageUrls, memoImages, memoImageCount]);

  const buildCopyText = () => {
    const trimmedText = memoText;
    if (!trimmedText) return "";
    return memo.text;
  };

  const handleCopy = async () => {
    const content = buildCopyText();
    const canCopyImages =
      typeof window !== "undefined" &&
      Boolean(navigator.clipboard?.write) &&
      typeof ClipboardItem !== "undefined";
    const supportedImageMimeTypes = getSupportedClipboardImageMimeTypes();

    let didCopy = false;

    try {
      if (canCopyImages) {
        const imageUrls = await resolveCopyImageUrls();
        for (const imageUrl of imageUrls) {
          try {
            const response = await fetch(imageUrl);
            if (!response.ok) {
              throw new Error(`Failed to fetch image: ${response.status}`);
            }
            const blob = await response.blob();
            const prepared = await prepareClipboardImageBlobs(
              blob,
              supportedImageMimeTypes,
            );
            await navigator.clipboard.write([
              new ClipboardItem(prepared),
            ]);
            didCopy = true;
          } catch (error) {
            console.error("Failed to copy memo image:", error);
          }
        }
      }

      if (content) {
        try {
          didCopy = (await copyTextToClipboard(content)) || didCopy;
        } catch (error) {
          console.error("Failed to copy memo text:", error);
        }
      }

      if (didCopy) {
        setIsCopied(true);
        if (copyResetTimeoutRef.current) {
          window.clearTimeout(copyResetTimeoutRef.current);
        }
        copyResetTimeoutRef.current = window.setTimeout(() => {
          setIsCopied(false);
          copyResetTimeoutRef.current = null;
        }, 1000);
      }
    } catch (error) {
      console.error("Failed to copy memo:", error);
    }
  };

  const handleImageClick = (index: number) => {
    preloadMemoImagePreview();
    setExpandedImageIndex((current) => (current === index ? null : index));
  };

  return (
    <>
      <div
        className="group bg-[#FAF8F7] px-2 py-3 border-b border-tertiary opacity-0 animate-blur-fade-slide-in transition-colors"
        onDoubleClick={handleCardDoubleClick}
      >
        <div className="flex flex-col gap-1">
          <div className="flex h-4 items-start justify-between gap-4 overflow-visible leading-4">
            <div className="flex items-center gap-2">
              <p className="text-[13px] text-[#77777780]">
                {displayTimestampLabel}
              </p>
              {isPending ? (
                <span className="rounded bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground">
                  Pending
                </span>
              ) : null}
            </div>
            <MemoCardActionsSlot
              actionsLoaded={actionsLoaded}
              actionsOpen={actionsOpen}
              onActionsOpenChange={setActionsOpen}
              isTrash={isTrash}
              isDeleting={isDeleting}
              isRestoring={isRestoring}
              isCopied={isCopied}
              hasCopyContent={hasCopyContent}
              onCopy={handleCopy}
              onEdit={handleEdit}
              onDelete={handleDelete}
              onRestore={handleRestore}
              onActionsHover={handleActionsHover}
              onActionsClick={handleActionsClick}
            />
          </div>

          <MemoCardContent
            memoId={memo.id}
            memoText={memoText}
            linkifiedMemoText={linkifiedMemoText}
            contentId={contentId}
            contentRef={contentRef}
            shouldShowToggle={shouldShowToggle}
            displayImages={displayImages}
            memoImageCount={memoImageCount}
            needsImageFetch={needsImageFetch}
            isResolvingImages={isResolvingImages}
            onResolvePendingImages={resolvePendingImages}
            onImageClick={handleImageClick}
            onImageHover={preloadMemoImagePreview}
            imageAreaRef={imageAreaRef}
          />
        </div>
      </div>
      {expandedImageIndex !== null && displayImages.length > 0 ? (
        <MemoImagePreview
          imageUrls={displayImages}
          initialIndex={expandedImageIndex}
          onClose={() => setExpandedImageIndex(null)}
        />
      ) : null}
    </>
  );
}

function areMemoCardPropsEqual(prev: MemoCardProps, next: MemoCardProps) {
  if (
    prev.memo === next.memo &&
    prev.isTrash === next.isTrash &&
    prev.isOnline === next.isOnline &&
    prev.onEdit === next.onEdit &&
    prev.onDelete === next.onDelete &&
    prev.onRestore === next.onRestore
  ) {
    return true;
  }

  if (
    prev.isTrash !== next.isTrash ||
    prev.isOnline !== next.isOnline ||
    prev.onEdit !== next.onEdit ||
    prev.onDelete !== next.onDelete ||
    prev.onRestore !== next.onRestore
  ) {
    return false;
  }

  const prevMemo = prev.memo;
  const nextMemo = next.memo;

  if (prevMemo.id !== nextMemo.id) return false;
  if (prevMemo.text !== nextMemo.text) return false;
  if (prevMemo.created_at !== nextMemo.created_at) return false;
  if (prevMemo.updated_at !== nextMemo.updated_at) return false;
  if (prevMemo.deleted_at !== nextMemo.deleted_at) return false;
  if (prevMemo.version !== nextMemo.version) return false;
  if (prevMemo.hasImages !== nextMemo.hasImages) return false;
  if (prevMemo.imageCount !== nextMemo.imageCount) return false;
  const prevImages = prevMemo.images ?? [];
  const nextImages = nextMemo.images ?? [];
  return areImagesEqual(prevImages, nextImages);
}

export const MemoCard = reactMemo(MemoCardBase, areMemoCardPropsEqual);
