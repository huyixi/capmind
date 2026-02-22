"use client";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Check, Copy, Pencil, RotateCcw, Trash2 } from "lucide-react";
import { MemoCardActionsTrigger } from "./actions-trigger";

interface MemoCardActionsProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  isTrash: boolean;
  isDeleting: boolean;
  isRestoring: boolean;
  isCopied: boolean;
  hasCopyContent: boolean;
  onCopy: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onRestore?: () => void;
}

export function MemoCardActions({
  open,
  onOpenChange,
  isTrash,
  isDeleting,
  isRestoring,
  isCopied,
  hasCopyContent,
  onCopy,
  onEdit,
  onDelete,
  onRestore,
}: MemoCardActionsProps) {
  const copyItem = (
    <DropdownMenuItem onClick={onCopy} disabled={!hasCopyContent}>
      {isCopied ? (
        <Check className="h-4 w-4 text-emerald-500 transition-colors" />
      ) : (
        <Copy className="h-4 w-4 transition-colors" />
      )}
      Copy
    </DropdownMenuItem>
  );

  return (
    <DropdownMenu open={open} onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>
        <MemoCardActionsTrigger disabled={isDeleting || isRestoring} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {isTrash ? (
          <>
            {copyItem}
            <DropdownMenuItem onClick={onRestore} disabled={isRestoring}>
              <RotateCcw className="h-4 w-4" />
              Restore
            </DropdownMenuItem>
          </>
        ) : (
          <>
            {copyItem}
            <DropdownMenuItem onClick={onEdit} disabled={isDeleting}>
              <Pencil className="h-4 w-4" />
              Edit
            </DropdownMenuItem>
            <DropdownMenuItem
              variant="destructive"
              onClick={onDelete}
              disabled={isDeleting}
            >
              <Trash2 className="h-4 w-4" />
              Delete
            </DropdownMenuItem>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
