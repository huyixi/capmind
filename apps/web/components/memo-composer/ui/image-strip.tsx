"use client";

import { memo } from "react";
import { X } from "lucide-react";

export interface ImageStripItem {
  kind: "existing" | "new";
  url: string;
  index: number;
}

export interface ImageStripProps {
  items: ImageStripItem[];
  canManageImages: boolean;
  onRemoveExisting: (index: number) => void;
  onRemoveNew: (index: number) => void;
}

export const ImageStrip = memo(function ImageStrip({
  items,
  canManageImages,
  onRemoveExisting,
  onRemoveNew,
}: ImageStripProps) {
  if (items.length === 0) return null;

  return (
    <div className="flex min-w-0 items-center gap-2 shrink-0">
      <div className="composer-scroll flex min-w-0 flex-1 flex-nowrap items-center gap-2 overflow-x-auto overflow-y-hidden">
        {items.map((item, itemIndex) => (
          <div
            key={`${item.kind}-${item.url}-${itemIndex}`}
            className="relative h-16 w-16 shrink-0 overflow-hidden rounded-md border border-border bg-muted/30"
          >
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={item.url}
              alt={`预览 ${itemIndex + 1}`}
              className="h-full w-full object-cover"
            />
            {canManageImages ? (
              <button
                type="button"
                onClick={() =>
                  item.kind === "existing"
                    ? onRemoveExisting(item.index)
                    : onRemoveNew(item.index)
                }
                className="absolute right-1 top-1 inline-flex size-5 items-center justify-center rounded-full bg-white/90 text-foreground shadow-sm"
              >
                <X className="h-3 w-3" />
                <span className="sr-only">移除图片</span>
              </button>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  );
});
