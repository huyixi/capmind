export type MemoComposerSubmitResult = {
  ok: boolean;
  error?: string;
  reason?: "auth" | "network" | "unknown";
};

export interface MemoComposerProps {
  onSubmit: (payload: {
    text: string;
    images: File[];
    existingImageUrls: string[];
  }) =>
    | MemoComposerSubmitResult
    | Promise<MemoComposerSubmitResult>
    | void
    | Promise<void>;
  maxImages?: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mode?: "create" | "edit";
  initialText?: string;
  allowImages?: boolean;
  initialImages?: string[];
  hasFallbackImages?: boolean;
  submitLabel?: string;
  placeholder?: string;
  title?: string;
  onDraftTextChange?: (value: string) => void;
  onDraftClear?: () => void;
  onComposerFocus?: () => void;
  onComposerFirstKeystroke?: () => void;
}
