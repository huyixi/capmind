"use client";

import { Plus } from "lucide-react";

interface MemoCreateButtonProps {
  onClick: () => void;
  onPointerEnter?: () => void;
  srLabel: string;
}

export function MemoCreateButton({
  onClick,
  onPointerEnter,
  srLabel,
}: MemoCreateButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      onPointerEnter={onPointerEnter}
      aria-label={srLabel}
      className="fixed bottom-6 right-6 z-30 inline-flex size-14 items-center justify-center rounded-[12px] bg-brand text-white transition-colors duration-150 hover:cursor-pointer hover:bg-brand-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand"
    >
      <Plus className="size-6" aria-hidden="true" />
      <span className="sr-only">{srLabel}</span>
    </button>
  );
}
