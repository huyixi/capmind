"use client";

import type { ReactNode } from "react";
import { FileText } from "lucide-react";

interface MemoListEmptyStateProps {
  isTrash: boolean;
  title?: string;
  description?: string;
  action?: ReactNode;
}

export function MemoListEmptyState({
  isTrash,
  title,
  description,
  action,
}: MemoListEmptyStateProps) {
  const fallbackTitle = isTrash ? "Trash is empty" : "No memos yet";
  const fallbackDescription = isTrash
    ? "Deleted memos will show up here. Restore them to bring them back."
    : "Start by writing your first memo below. You can add text and images to capture your thoughts.";
  const resolvedTitle = title ?? fallbackTitle;
  const resolvedDescription = description ?? fallbackDescription;

  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-secondary mb-4">
        <FileText className="h-8 w-8 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-medium text-foreground mb-1">
        {resolvedTitle}
      </h3>
      <p className="text-sm text-muted-foreground max-w-sm">
        {resolvedDescription}
      </p>
      {action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}
