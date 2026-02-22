"use client";

import { useState } from "react";
import { Memo } from "@/lib/types";
import {
  EXPORT_OPTIONS,
  ExportRange,
  buildExportText,
  filterMemosByRange,
  formatExportFileName,
} from "@/lib/memo-export";

const EXPORT_PAGE_SIZE = 100;

const downloadTextFile = (content: string, filename: string) => {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
};

const copyTextToClipboard = async (content: string) => {
  if (navigator?.clipboard?.writeText) {
    await navigator.clipboard.writeText(content);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = content;
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.focus();
  textarea.select();
  document.execCommand("copy");
  textarea.remove();
};

const fetchAllMemos = async () => {
  const memos: Memo[] = [];
  let page = 0;
  let shouldContinue = true;

  while (shouldContinue) {
    const params = new URLSearchParams();
    params.set("page", page.toString());
    params.set("pageSize", EXPORT_PAGE_SIZE.toString());
    const response = await fetch(`/api/memos?${params.toString()}`, {
      credentials: "include",
      cache: "no-store",
      headers: { "cache-control": "no-cache" },
    });

    if (!response.ok) {
      throw new Error(response.statusText);
    }

    const payload = await response.json();
    const pageMemos = (payload.memos ?? []) as Memo[];
    memos.push(...pageMemos);
    if (pageMemos.length < EXPORT_PAGE_SIZE) {
      shouldContinue = false;
    } else {
      page += 1;
    }
  }

  return memos;
};

export function useMemoExport() {
  const [exportingRange, setExportingRange] = useState<ExportRange | null>(
    null,
  );
  const [exportError, setExportError] = useState<string | null>(null);
  const isExporting = exportingRange !== null;

  const runExportTask = async (
    range: ExportRange,
    onSuccess: (content: string) => Promise<void> | void,
  ) => {
    if (isExporting) return false;
    setExportError(null);
    setExportingRange(range);

    try {
      const memos = await fetchAllMemos();
      const filtered = filterMemosByRange(memos, range);
      const content = buildExportText(filtered);

      if (!content) {
        setExportError("No memos to export for this time range.");
        return false;
      }

      await onSuccess(content);
      return true;
    } catch (error) {
      console.error("Failed to export memos:", error);
      setExportError("Failed to export memos.");
      return false;
    } finally {
      setExportingRange(null);
    }
  };

  const exportMemos = async (range: ExportRange) =>
    runExportTask(range, (content) => {
      downloadTextFile(content, formatExportFileName(new Date()));
    });

  const copyMemos = async (range: ExportRange) =>
    runExportTask(range, (content) => copyTextToClipboard(content));

  const clearExportError = () => setExportError(null);

  return {
    exportOptions: EXPORT_OPTIONS,
    exportError,
    exportingRange,
    isExporting,
    exportMemos,
    copyMemos,
    clearExportError,
  };
}
