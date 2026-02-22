"use client";

import type { ComponentProps } from "react";
import { MoreHorizontal } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type MemoCardActionsTriggerProps = Omit<
  ComponentProps<typeof Button>,
  "variant" | "size"
> & {
  onHover?: () => void;
};

export function MemoCardActionsTrigger({
  disabled,
  onHover,
  onMouseEnter,
  onFocus,
  className,
  ...props
}: MemoCardActionsTriggerProps) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className={cn(
        "h-8 w-8 -my-2 text-muted-foreground hover:text-foreground hover:bg-secondary",
        className,
      )}
      aria-label="Open memo actions"
      disabled={disabled}
      onMouseEnter={(event) => {
        onMouseEnter?.(event);
        onHover?.();
      }}
      onFocus={(event) => {
        onFocus?.(event);
        onHover?.();
      }}
      {...props}
    >
      <MoreHorizontal className="h-4 w-4" />
    </Button>
  );
}
