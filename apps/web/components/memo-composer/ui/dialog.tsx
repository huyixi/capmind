"use client";

import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { MemoComposerForm } from "./form";
import { useMemoComposer } from "../logic/use-memo-composer";
import type { MemoComposerProps } from "../logic/types";

export function MemoComposerDialog(props: MemoComposerProps) {
  const composer = useMemoComposer(props);

  return (
    <Dialog open={composer.open} onOpenChange={composer.handleOpenChange}>
      <DialogContent
        className={[
          "p-0 overflow-hidden",
          composer.isFullscreen
            ? "inset-0 h-full w-full max-w-none translate-x-0 translate-y-0 rounded-none border-none bg-white sm:max-w-none"
            : "inset-0 h-full w-full max-w-none translate-x-0 translate-y-0 rounded-none border-none bg-white sm:top-[50%] sm:left-[50%] sm:right-auto sm:bottom-auto sm:h-auto sm:max-h-[85vh] sm:w-full sm:max-w-lg sm:translate-x-[-50%] sm:translate-y-[-50%] sm:rounded-xl sm:border sm:border-border/40 sm:bg-white sm:shadow-lg",
        ].join(" ")}
        overlayClassName={composer.isFullscreen ? "bg-white" : undefined}
        showCloseButton={false}
        onOpenAutoFocus={composer.handleOpenAutoFocus}
      >
        <DialogTitle className="sr-only">{composer.resolvedTitle}</DialogTitle>
        <MemoComposerForm
          text={composer.text}
          textareaRef={composer.textareaRef}
          fileInputRef={composer.fileInputRef}
          placeholder={composer.resolvedPlaceholder}
          isFullscreen={composer.isFullscreen}
          canExpand={composer.canExpand}
          canManageImages={composer.canManageImages}
          maxImages={composer.maxImages}
          totalImages={composer.totalImages}
          isSubmitting={composer.isSubmitting}
          canSubmit={composer.canSubmit}
          submitLabel={composer.resolvedSubmitLabel}
          imageItems={composer.imageItems}
          submitError={composer.submitError}
          onTextChange={composer.handleTextChange}
          onKeyDown={composer.handleKeyDown}
          onAddImage={composer.handleAddImageClick}
          onCancel={composer.handleCancel}
          onSubmit={composer.handleSubmit}
          onToggleFullscreen={composer.toggleFullscreen}
          onImageSelect={composer.handleImageSelect}
          onRemoveExistingImage={composer.handleRemoveExistingImage}
          onRemoveImage={composer.handleRemoveImage}
          onTextareaFocus={composer.handleTextareaFocus}
        />
      </DialogContent>
    </Dialog>
  );
}
