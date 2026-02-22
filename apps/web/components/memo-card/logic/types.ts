import { Memo } from "@/lib/types";

export interface MemoCardProps {
  memo: Memo;
  onDelete: (
    id: string,
    images: string[],
    expectedVersion: string,
  ) => Promise<void>;
  onEdit?: (memo: Memo) => void;
  onRestore?: (memo: Memo) => Promise<boolean>;
  isTrash?: boolean;
  isOnline: boolean;
}
