import { useEffect, type MutableRefObject } from "react";
import type { MemoSearchActions } from "@/components/memo-list/logic/container";

interface UseMemoShortcutsOptions {
  searchActionsRef: MutableRefObject<MemoSearchActions | null>;
  resetComposerState: () => void;
  openCreateComposer: () => void;
}

function isEditableTarget(target: EventTarget | null) {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
  );
}

export function useMemoShortcuts({
  searchActionsRef,
  resetComposerState,
  openCreateComposer,
}: UseMemoShortcutsOptions) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      if (event.isComposing) return;

      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        const searchActions = searchActionsRef.current;
        if (!searchActions) return;
        event.preventDefault();
        resetComposerState();
        searchActions.openSearch();
        return;
      }

      if (isEditableTarget(event.target)) {
        return;
      }

      if (!event.metaKey && !event.ctrlKey && !event.altKey) {
        if (event.key.toLowerCase() === "f") {
          const searchActions = searchActionsRef.current;
          if (!searchActions) return;
          event.preventDefault();
          resetComposerState();
          searchActions.openSearch();
          return;
        }

        if (event.key.toLowerCase() === "c") {
          event.preventDefault();
          searchActionsRef.current?.closeSearch();
          openCreateComposer();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [openCreateComposer, resetComposerState, searchActionsRef]);
}
