"use client";

import { useEffect, useState } from "react";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { X } from "lucide-react";

interface MemoImagePreviewProps {
  imageUrls: string[];
  initialIndex: number;
  onClose: () => void;
}

const PLACEHOLDER_SRC = "/placeholder.svg";
const getSafeImageSrc = (url: string) => {
  if (typeof url !== "string") return PLACEHOLDER_SRC;
  const trimmed = url.trim();
  return trimmed.length > 0 ? trimmed : PLACEHOLDER_SRC;
};

const resolveImage = (url: string) => {
  const src = getSafeImageSrc(url);
  return { src };
};

const clampIndex = (index: number, length: number) => {
  if (!Number.isFinite(index)) return 0;
  if (length <= 0) return 0;
  if (index < 0) return 0;
  if (index >= length) return length - 1;
  return index;
};

export function MemoImagePreview({
  imageUrls,
  initialIndex,
  onClose,
}: MemoImagePreviewProps) {
  const safeImageUrls = Array.isArray(imageUrls) ? imageUrls : [];
  const [activeIndex, setActiveIndex] = useState(() =>
    clampIndex(initialIndex, safeImageUrls.length),
  );

  useEffect(() => {
    setActiveIndex((current) => {
      if (safeImageUrls.length === 0) return 0;
      const hasValidInitialIndex =
        Number.isFinite(initialIndex) &&
        initialIndex >= 0 &&
        initialIndex < safeImageUrls.length;
      if (hasValidInitialIndex && initialIndex !== current) {
        return initialIndex;
      }
      if (!Number.isFinite(current) || current < 0) return 0;
      if (current >= safeImageUrls.length) return safeImageUrls.length - 1;
      return current;
    });
  }, [safeImageUrls.length, initialIndex]);

  const activeUrl = safeImageUrls[activeIndex] ?? "";
  const activeImage = resolveImage(activeUrl);
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent
        showCloseButton={false}
        className="!fixed !inset-0 !h-[100dvh] !w-[100dvw] !max-w-none !max-h-none !translate-x-0 !translate-y-0 rounded-none border-none bg-black p-0 shadow-none"
      >
        <DialogTitle className="sr-only">Image preview</DialogTitle>
        <button
          type="button"
          onClick={onClose}
          className="absolute left-4 top-[calc(env(safe-area-inset-top)+1rem)] z-[60] inline-flex h-9 w-9 items-center justify-center rounded-full bg-white/30 text-white transition hover:bg-white/50"
          aria-label="Close image preview"
        >
          <X className="h-4 w-4" />
        </button>
        <div className="flex h-full w-full flex-col">
          <div className="flex min-h-0 flex-1 items-center justify-center p-4">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={activeImage.src}
              alt="Expanded memo image"
              className="h-auto w-auto max-h-[min(88vh,900px)] max-w-[min(92vw,1200px)] object-contain"
            />
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
