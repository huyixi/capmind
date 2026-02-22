"use client";

import { Suspense, lazy } from "react";
import { MemoCardActionsTrigger } from "./actions-trigger";

const MemoCardActions = lazy(() =>
  import("./actions").then((mod) => ({
    default: mod.MemoCardActions,
  })),
);

export const preloadMemoCardActions = () => {
  void import("./actions");
};

interface MemoCardActionsSlotProps {
  actionsLoaded: boolean;
  actionsOpen: boolean;
  onActionsOpenChange: (open: boolean) => void;
  isTrash: boolean;
  isDeleting: boolean;
  isRestoring: boolean;
  isCopied: boolean;
  hasCopyContent: boolean;
  onCopy: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onRestore: () => void;
  onActionsHover: () => void;
  onActionsClick: () => void;
}

export function MemoCardActionsSlot({
  actionsLoaded,
  actionsOpen,
  onActionsOpenChange,
  isTrash,
  isDeleting,
  isRestoring,
  isCopied,
  hasCopyContent,
  onCopy,
  onEdit,
  onDelete,
  onRestore,
  onActionsHover,
  onActionsClick,
}: MemoCardActionsSlotProps) {
  const isBusy = isDeleting || isRestoring;

  return (
    <div className="flex items-center gap-1 opacity-60">
      {actionsLoaded ? (
        <Suspense
          fallback={
            <MemoCardActionsTrigger
              disabled={isBusy}
              onHover={onActionsHover}
              onClick={onActionsClick}
            />
          }
        >
          <MemoCardActions
            open={actionsOpen}
            onOpenChange={onActionsOpenChange}
            isTrash={isTrash}
            isDeleting={isDeleting}
            isRestoring={isRestoring}
            isCopied={isCopied}
            hasCopyContent={hasCopyContent}
            onCopy={onCopy}
            onEdit={onEdit}
            onDelete={onDelete}
            onRestore={onRestore}
          />
        </Suspense>
      ) : (
        <MemoCardActionsTrigger
          disabled={isBusy}
          onHover={onActionsHover}
          onClick={onActionsClick}
        />
      )}
    </div>
  );
}
