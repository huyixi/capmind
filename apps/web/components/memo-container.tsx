"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { type AuthUser as User } from "@supabase/supabase-js";
import { useOnlineStatus } from "@/hooks/use-online-status";
import { useMemoComposerController } from "@/hooks/use-memo-composer-controller";
import { useMemoShortcuts } from "@/hooks/use-memo-shortcuts";
import { MemoListShell } from "@/components/memo-list/logic/shell";
import { MemoCreateButton } from "@/components/memo-create-button";
import { MemoComposerPanel } from "@/components/memo-composer/ui/panel";
import type { MemoSearchActions } from "@/components/memo-list/logic/container";
import { reportComposerPerfMetric } from "@/lib/composer-performance";
import type { Memo } from "@/lib/types";

const DRAFT_STORAGE_KEY = "memo-draft:create";
const STARTUP_BACKGROUND_DELAY_MS = 2000;

interface MemoContainerProps {
  initialUser: User | null;
  initialMemos?: Memo[];
}

export function MemoContainer({
  initialUser,
  initialMemos,
}: MemoContainerProps) {
  const [shouldMountMemoList, setShouldMountMemoList] = useState(false);
  const [isBackgroundWorkEnabled, setIsBackgroundWorkEnabled] = useState(false);
  const isOnline = useOnlineStatus();
  const composerOpenAtRef = useRef<number | null>(null);
  const hasReportedComposerFocusRef = useRef(false);
  const hasReportedFirstKeystrokeRef = useRef(false);

  const resolveSubmitUser = useCallback(async (): Promise<User | null> => {
    if (initialUser) return initialUser;
    try {
      const { createClient } = await import("@/lib/supabase/client");
      const supabase = createClient();
      const { data, error } = await supabase.auth.getSession();
      if (error || !data?.session?.user) return null;
      return data.session.user;
    } catch {
      return null;
    }
  }, [initialUser]);

  const {
    isComposerOpen,
    composerMode,
    editingMemo,
    editingImages,
    canEditImages,
    draftText,
    handleDraftTextChange,
    clearDraftText,
    openCreateComposer,
    handleEditOpen: handleEditOpenRaw,
    handleComposerOpenChange,
    handleComposerSubmit,
    resetComposerState,
    registerComposerActions,
  } = useMemoComposerController({
    isOnline,
    resolveSubmitUser,
    draftStorageKey: DRAFT_STORAGE_KEY,
  });
  const searchActionsRef = useRef<MemoSearchActions | null>(null);

  const registerSearchActions = useCallback(
    (actions: MemoSearchActions | null) => {
      searchActionsRef.current = actions;
    },
    [],
  );

  useEffect(() => {
    if (!isComposerOpen) {
      composerOpenAtRef.current = null;
      hasReportedComposerFocusRef.current = false;
      hasReportedFirstKeystrokeRef.current = false;
      return;
    }

    composerOpenAtRef.current = performance.now();
    hasReportedComposerFocusRef.current = false;
    hasReportedFirstKeystrokeRef.current = false;
  }, [isComposerOpen]);

  const handleComposerFocus = useCallback(() => {
    if (hasReportedComposerFocusRef.current) return;
    const openedAt = composerOpenAtRef.current;
    if (openedAt === null) return;
    hasReportedComposerFocusRef.current = true;
    reportComposerPerfMetric(
      "composer_open_to_focus_ms",
      performance.now() - openedAt,
      composerMode,
    );
  }, [composerMode]);

  const handleComposerFirstKeystroke = useCallback(() => {
    if (hasReportedFirstKeystrokeRef.current) return;
    const openedAt = composerOpenAtRef.current;
    if (openedAt === null) return;
    hasReportedFirstKeystrokeRef.current = true;
    reportComposerPerfMetric(
      "first_keystroke_ready_ms",
      performance.now() - openedAt,
      composerMode,
    );
  }, [composerMode]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setIsBackgroundWorkEnabled(true);
      setShouldMountMemoList(true);
    }, STARTUP_BACKGROUND_DELAY_MS);

    return () => {
      window.clearTimeout(timer);
    };
  }, []);

  const enableBackgroundWorkNow = useCallback(() => {
    setIsBackgroundWorkEnabled((current) => {
      if (current) return current;
      setShouldMountMemoList(true);
      return true;
    });
  }, []);

  const handleComposerDraftChange = useCallback(
    (value: string) => {
      if (value.length > 0) {
        enableBackgroundWorkNow();
      }
      handleDraftTextChange(value);
    },
    [enableBackgroundWorkNow, handleDraftTextChange],
  );

  const handleCreateOpen = useCallback(() => {
    openCreateComposer();
  }, [openCreateComposer]);

  const handleEditOpen = useCallback(
    (memo: Memo) => {
      enableBackgroundWorkNow();
      void handleEditOpenRaw(memo);
    },
    [enableBackgroundWorkNow, handleEditOpenRaw],
  );

  useEffect(() => {
    if (shouldMountMemoList) {
      return;
    }
    const hasCreateDraft = draftText.trim().length > 0;
    if (hasCreateDraft) {
      setShouldMountMemoList(true);
      setIsBackgroundWorkEnabled(true);
    }
  }, [draftText, shouldMountMemoList]);

  useMemoShortcuts({
    searchActionsRef,
    resetComposerState,
    openCreateComposer: handleCreateOpen,
  });

  return (
    <div className="min-h-screen flex flex-col bg-background">
      {shouldMountMemoList ? (
        <MemoListShell
          initialUser={initialUser}
          initialMemos={initialMemos}
          backgroundWorkEnabled={isBackgroundWorkEnabled}
          onEdit={handleEditOpen}
          onRegisterComposerActions={registerComposerActions}
          onRegisterSearchActions={registerSearchActions}
          onResetComposer={resetComposerState}
        />
      ) : null}

      <MemoCreateButton
        onClick={handleCreateOpen}
        srLabel="新建 Memo"
      />

      <MemoComposerPanel
        open={isComposerOpen}
        onOpenChange={handleComposerOpenChange}
        onSubmit={handleComposerSubmit}
        mode={composerMode}
        editingMemo={editingMemo}
        editingImages={editingImages}
        canEditImages={canEditImages}
        draftText={draftText}
        onDraftTextChange={
          composerMode === "create" ? handleComposerDraftChange : undefined
        }
        onDraftClear={composerMode === "create" ? clearDraftText : undefined}
        onComposerFocus={handleComposerFocus}
        onComposerFirstKeystroke={handleComposerFirstKeystroke}
      />
    </div>
  );
}
