import { useCallback, useEffect, useRef, useState } from "react";

const PERSIST_DEBOUNCE_MS = 250;

export function useDraftStorage(storageKey: string) {
  const [draftText, setDraftText] = useState("");
  const pendingValueRef = useRef<string | null>(null);
  const persistTimeoutRef = useRef<number | null>(null);

  const writeDraftText = useCallback(
    (value: string) => {
      if (typeof window === "undefined") return;
      try {
        if (!value.trim()) {
          window.localStorage.removeItem(storageKey);
        } else {
          window.localStorage.setItem(storageKey, value);
        }
      } catch {
        // Ignore storage errors to avoid blocking the composer.
      }
    },
    [storageKey],
  );

  const cancelPendingPersist = useCallback(() => {
    if (typeof window === "undefined") return;
    if (persistTimeoutRef.current !== null) {
      window.clearTimeout(persistTimeoutRef.current);
      persistTimeoutRef.current = null;
    }
  }, []);

  const schedulePersist = useCallback(
    (value: string) => {
      if (typeof window === "undefined") return;
      pendingValueRef.current = value;
      cancelPendingPersist();
      persistTimeoutRef.current = window.setTimeout(() => {
        const nextValue = pendingValueRef.current ?? "";
        pendingValueRef.current = null;
        writeDraftText(nextValue);
        persistTimeoutRef.current = null;
      }, PERSIST_DEBOUNCE_MS);
    },
    [cancelPendingPersist, writeDraftText],
  );

  const readDraftText = useCallback(() => {
    if (typeof window === "undefined") return "";
    try {
      return window.localStorage.getItem(storageKey) ?? "";
    } catch {
      return "";
    }
  }, [storageKey]);

  const loadDraftText = useCallback(() => {
    setDraftText(readDraftText());
  }, [readDraftText]);

  const handleDraftTextChange = useCallback(
    (value: string) => {
      setDraftText(value);
      schedulePersist(value);
    },
    [schedulePersist],
  );

  const clearDraftText = useCallback(() => {
    setDraftText("");
    pendingValueRef.current = null;
    cancelPendingPersist();
    writeDraftText("");
  }, [cancelPendingPersist, writeDraftText]);

  useEffect(() => {
    return () => {
      if (pendingValueRef.current !== null) {
        writeDraftText(pendingValueRef.current);
        pendingValueRef.current = null;
      }
      cancelPendingPersist();
    };
  }, [cancelPendingPersist, writeDraftText]);

  return { draftText, loadDraftText, handleDraftTextChange, clearDraftText };
}
