"use client";

import { memo } from "react";
import { ImagePlus } from "lucide-react";
import { Button } from "@/components/ui/button";

export interface ActionRowProps {
  variant: "mobile" | "desktop";
  canManageImages: boolean;
  totalImages: number;
  maxImages: number;
  isSubmitting: boolean;
  canSubmit: boolean;
  submitLabel: string;
  onAddImage: () => void;
  onCancel: () => void;
  onSubmit: () => void;
}

export const ActionRow = memo(function ActionRow({
  variant,
  canManageImages,
  totalImages,
  maxImages,
  isSubmitting,
  canSubmit,
  submitLabel,
  onAddImage,
  onCancel,
  onSubmit,
}: ActionRowProps) {
  const isMobile = variant === "mobile";

  return (
    <div
      className={
        isMobile
          ? "flex items-center justify-between gap-3 border-b px-4 py-2 sm:hidden"
          : "hidden items-center justify-between shrink-0 sm:flex"
      }
    >
      <div className="flex items-center">
        {canManageImages ? (
          <Button
            type="button"
            variant="ghost"
            onClick={onAddImage}
            disabled={totalImages >= maxImages}
            className="h-10 w-10 text-muted-foreground hover:text-foreground"
          >
            <ImagePlus className="h-5 w-5" />
            <span className="sr-only">Add image</span>
          </Button>
        ) : isMobile ? null : (
          <div />
        )}
      </div>
      <div className="flex items-center gap-2">
        <Button
          type="button"
          variant="ghost"
          onClick={onCancel}
          disabled={isSubmitting}
          className="text-muted-foreground hover:text-foreground hover:cursor-pointer"
        >
          Cancel
        </Button>
        <Button
          type="button"
          onClick={onSubmit}
          disabled={!canSubmit || isSubmitting}
          className="bg-brand hover:bg-brand-hover hover:cursor-pointer"
        >
          {submitLabel}
        </Button>
      </div>
    </div>
  );
});
