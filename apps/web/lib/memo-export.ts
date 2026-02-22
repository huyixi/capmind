import { Memo } from "@/lib/types";

export type ExportRange = "day" | "week" | "all";

export const EXPORT_OPTIONS: { range: ExportRange; label: string }[] = [
  { range: "day", label: "Today" },
  { range: "week", label: "Last 7 days" },
  { range: "all", label: "All" },
];

const MS_IN_DAY = 24 * 60 * 60 * 1000;

export const formatExportFileName = (date: Date) => {
  const pad = (value: number) => value.toString().padStart(2, "0");
  const year = date.getFullYear();
  const month = pad(date.getMonth() + 1);
  const day = pad(date.getDate());
  const hour = pad(date.getHours());
  const minute = pad(date.getMinutes());
  return `capmind-${year}${month}${day}-${hour}${minute}.txt`;
};

export const buildExportText = (memos: Memo[]) =>
  memos
    .map((memo) => memo.text ?? "")
    .filter((text) => text.trim().length > 0)
    .join("\n\n");

const isDeletedMemo = (memo: Memo & { deletedAt?: string | null }) =>
  memo.deleted_at != null || memo.deletedAt != null;

export const filterMemosByRange = (memos: Memo[], range: ExportRange) => {
  if (range === "all") {
    return memos.filter((memo) => !isDeletedMemo(memo));
  }

  const days = range === "day" ? 1 : 7;
  const cutoff = Date.now() - days * MS_IN_DAY;

  return memos.filter((memo) => {
    if (isDeletedMemo(memo)) return false;
    const updatedAt = new Date(memo.updated_at).getTime();
    if (!Number.isFinite(updatedAt)) return false;
    return updatedAt >= cutoff;
  });
};
