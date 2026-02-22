"use client";

import type { ChangeEvent } from "react";

interface MemoComposerPendingPanelProps {
  open: boolean;
  mode: "create" | "edit";
  draftText: string;
  onOpenChange: (open: boolean) => void;
  onDraftTextChange?: (value: string) => void;
}

export function MemoComposerPendingPanel({
  open,
  mode,
  draftText,
  onOpenChange,
  onDraftTextChange,
}: MemoComposerPendingPanelProps) {
  if (!open) return null;

  const isCreateMode = mode === "create";
  const handleTextChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
    onDraftTextChange?.(event.target.value);
  };

  return (
    <div className="fixed inset-0 z-50">
      <button
        type="button"
        aria-label="Close composer"
        className="absolute inset-0 bg-black/45"
        onClick={() => onOpenChange(false)}
      />
      <div className="absolute inset-x-0 bottom-0 z-10 w-full rounded-t-xl border border-border/40 bg-white px-4 pb-5 pt-3 sm:left-1/2 sm:top-1/2 sm:h-auto sm:max-h-[85vh] sm:w-full sm:max-w-lg sm:-translate-x-1/2 sm:-translate-y-1/2 sm:rounded-xl">
        <div className="mb-2 flex items-center justify-between">
          <p className="text-sm font-medium text-foreground">
            {isCreateMode ? "New memo" : "Opening editor..."}
          </p>
          <button
            type="button"
            className="text-sm text-muted-foreground hover:text-foreground"
            onClick={() => onOpenChange(false)}
          >
            Cancel
          </button>
        </div>
        {isCreateMode ? (
          <textarea
            autoFocus
            value={draftText}
            onChange={handleTextChange}
            placeholder="What's on your mind?"
            className="min-h-44 w-full resize-none rounded-md border border-border bg-white p-3 text-base outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
          />
        ) : (
          <div className="rounded-md border border-border bg-muted/30 px-3 py-10 text-center text-sm text-muted-foreground">
            Preparing editor...
          </div>
        )}
        <div className="mt-3 flex items-center justify-end gap-2">
          <button
            type="button"
            disabled
            className="rounded-md bg-brand px-4 py-2 text-sm font-medium text-white opacity-60"
          >
            Loading...
          </button>
        </div>
      </div>
    </div>
  );
}
