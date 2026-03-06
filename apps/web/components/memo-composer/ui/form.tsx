"use client";

import type { ChangeEvent, ClipboardEvent, KeyboardEvent, RefObject } from "react";
import { Maximize2, Minimize2 } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";
import { ActionRow } from "./action-row";
import { ImageStrip } from "./image-strip";
import type { MemoComposerImageItem } from "../logic/use-memo-composer";

export interface MemoComposerFormProps {
  text: string;
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  fileInputRef: RefObject<HTMLInputElement | null>;
  placeholder: string;
  isFullscreen: boolean;
  canExpand: boolean;
  canManageImages: boolean;
  maxImages: number;
  totalImages: number;
  isSubmitting: boolean;
  canSubmit: boolean;
  submitLabel: string;
  imageItems: MemoComposerImageItem[];
  submitError: string | null;
  onTextChange: (event: ChangeEvent<HTMLTextAreaElement>) => void;
  onKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  onImagePaste: (event: ClipboardEvent<HTMLTextAreaElement>) => void;
  onAddImage: () => void;
  onCancel: () => void;
  onSubmit: () => void;
  onToggleFullscreen: () => void;
  onImageSelect: (event: ChangeEvent<HTMLInputElement>) => void;
  onRemoveExistingImage: (index: number) => void;
  onRemoveImage: (index: number) => void;
  onTextareaFocus?: () => void;
}

export function MemoComposerForm({
  text,
  textareaRef,
  fileInputRef,
  placeholder,
  isFullscreen,
  canExpand,
  canManageImages,
  maxImages,
  totalImages,
  isSubmitting,
  canSubmit,
  submitLabel,
  imageItems,
  submitError,
  onTextChange,
  onKeyDown,
  onImagePaste,
  onAddImage,
  onCancel,
  onSubmit,
  onToggleFullscreen,
  onImageSelect,
  onRemoveExistingImage,
  onRemoveImage,
  onTextareaFocus,
}: MemoComposerFormProps) {
  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col bg-white">
      {canManageImages ? (
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          multiple
          onChange={onImageSelect}
          className="hidden"
        />
      ) : null}
      <div
        className={[
          "relative flex min-h-0 flex-1 flex-col",
          isFullscreen ? "sm:self-center sm:w-full sm:max-w-xl " : "",
        ].join(" ")}
      >
        {(canExpand || isFullscreen) && (
          <button
            type="button"
            onClick={onToggleFullscreen}
            className="absolute right-4 top-2 z-10 hidden sm:inline-flex size-7 items-center justify-center rounded-md border border-transparent bg-transparent text-muted-foreground/80 shadow-none transition-colors opacity-80 hover:bg-muted/70 hover:text-foreground/90 hover:opacity-100 focus-visible:border-border focus-visible:bg-muted/80"
            aria-label={isFullscreen ? "Exit full screen" : "Enter full screen"}
          >
            {isFullscreen ? (
              <Minimize2 className="h-4 w-4" />
            ) : (
              <Maximize2 className="h-4 w-4" />
            )}
          </button>
        )}
        <ActionRow
          variant="mobile"
          canManageImages={canManageImages}
          totalImages={totalImages}
          maxImages={maxImages}
          isSubmitting={isSubmitting}
          canSubmit={canSubmit}
          submitLabel={submitLabel}
          onAddImage={onAddImage}
          onCancel={onCancel}
          onSubmit={onSubmit}
        />
        <div
          className={[
            "flex min-h-0 flex-1 flex-col px-4 pb-5 pt-1",
            isFullscreen ? "sm:px-0" : "sm:min-h-96 sm:max-h-[85vh]",
          ].join(" ")}
        >
          <div
            className={[
              "composer-scroll flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto",
              isFullscreen ? "sm:bg-white sm:px-2 sm:py-3" : "",
            ].join(" ")}
          >
            <Textarea
              ref={textareaRef}
              autoFocus
              value={text}
              onChange={onTextChange}
              onKeyDown={onKeyDown}
              onPaste={onImagePaste}
              onFocus={onTextareaFocus}
              placeholder={placeholder}
              wrap="soft"
              style={{ fieldSizing: "fixed" }}
              className={[
                "composer-scroll flex-1 min-h-0 resize-none bg-white text-lg md:text-lg focus-visible:border-transparent focus-visible:ring-0 overflow-x-hidden",
                isFullscreen ? "sm:bg-transparent" : "",
              ].join(" ")}
            />
            <ImageStrip
              items={imageItems}
              canManageImages={canManageImages}
              onRemoveExisting={onRemoveExistingImage}
              onRemoveNew={onRemoveImage}
            />
          </div>
          <ActionRow
            variant="desktop"
            canManageImages={canManageImages}
            totalImages={totalImages}
            maxImages={maxImages}
            isSubmitting={isSubmitting}
            canSubmit={canSubmit}
            submitLabel={submitLabel}
            onAddImage={onAddImage}
            onCancel={onCancel}
            onSubmit={onSubmit}
          />
          {submitError ? (
            <p className="mt-2 text-sm text-red-500">{submitError}</p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
