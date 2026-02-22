"use client";

import dynamic from "next/dynamic";
import { useCallback, useEffect, useRef, useState } from "react";
import { type AuthUser as User } from "@supabase/supabase-js";
import { useOnlineStatus } from "@/hooks/use-online-status";
import { useMemoComposerController } from "@/hooks/use-memo-composer-controller";
import { useMemoShortcuts } from "@/hooks/use-memo-shortcuts";
import { MemoListShell } from "@/components/memo-list/logic/shell";
import { MemoCreateButton } from "@/components/memo-create-button";
import type { MemoComposerPanelProps } from "@/components/memo-composer/ui/panel";
import { MemoComposerPendingPanel } from "@/components/memo-composer/ui/pending-panel";
import type { MemoSearchActions } from "@/components/memo-list/logic/container";
import type { Memo } from "@/lib/types";

const DRAFT_STORAGE_KEY = "memo-draft:create";

type IdleCallbackWindow = Window & {
  requestIdleCallback?: (
    callback: () => void,
    options?: { timeout: number },
  ) => number;
  cancelIdleCallback?: (handle: number) => void;
};

const loadMemoComposerPanel = () =>
  import("@/components/memo-composer/ui/panel").then(
    (mod) => mod.MemoComposerPanel,
  );

let memoComposerPanelPromise: ReturnType<typeof loadMemoComposerPanel> | null =
  null;

const ensureMemoComposerPanelLoaded = () => {
  if (!memoComposerPanelPromise) {
    memoComposerPanelPromise = loadMemoComposerPanel();
  }
  return memoComposerPanelPromise;
};

const MemoComposerPanel = dynamic<MemoComposerPanelProps>(
  loadMemoComposerPanel,
  {
    ssr: false,
    loading: () => null,
  },
);

const preloadMemoComposerPanel = () => {
  void ensureMemoComposerPanelLoaded();
};

interface MemoContainerProps {
  initialUser: User | null;
  initialMemos?: Memo[];
}

export function MemoContainer({
  initialUser,
  initialMemos,
}: MemoContainerProps) {
  const [shouldMountMemoList, setShouldMountMemoList] = useState(false);
  const [isComposerPanelReady, setIsComposerPanelReady] = useState(false);
  const isOnline = useOnlineStatus();

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
    let cancelled = false;
    preloadMemoComposerPanel();
    void ensureMemoComposerPanelLoaded().then(() => {
      if (cancelled) return;
      setIsComposerPanelReady(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const mountMemoList = () => {
      if (cancelled) return;
      setShouldMountMemoList(true);
    };
    const idleWindow = window as IdleCallbackWindow;
    if (typeof idleWindow.requestIdleCallback === "function") {
      const idleHandle = idleWindow.requestIdleCallback(
        () => {
          mountMemoList();
        },
        { timeout: 1200 },
      );
      return () => {
        cancelled = true;
        if (typeof idleWindow.cancelIdleCallback === "function") {
          idleWindow.cancelIdleCallback(idleHandle);
        }
      };
    }
    const timeoutHandle = window.setTimeout(mountMemoList, 120);
    return () => {
      cancelled = true;
      window.clearTimeout(timeoutHandle);
    };
  }, []);

  const handleCreateOpen = useCallback(() => {
    preloadMemoComposerPanel();
    openCreateComposer();
  }, [openCreateComposer]);

  const handleCreateHover = useCallback(() => {
    preloadMemoComposerPanel();
  }, []);

  const handleEditOpen = useCallback(
    (memo: Memo) => {
      preloadMemoComposerPanel();
      void handleEditOpenRaw(memo);
    },
    [handleEditOpenRaw],
  );

  useMemoShortcuts({
    searchActionsRef,
    resetComposerState,
    openCreateComposer: handleCreateOpen,
  });

  const showPendingComposer = isComposerOpen && !isComposerPanelReady;

  return (
    <div className="min-h-screen flex flex-col bg-background">
      {shouldMountMemoList ? (
        <MemoListShell
          initialUser={initialUser}
          initialMemos={initialMemos}
          onEdit={handleEditOpen}
          onRegisterComposerActions={registerComposerActions}
          onRegisterSearchActions={registerSearchActions}
          onResetComposer={resetComposerState}
        />
      ) : null}

      <MemoCreateButton
        onClick={handleCreateOpen}
        onPointerEnter={handleCreateHover}
        srLabel="新建 Memo"
      />

      {showPendingComposer ? (
        <MemoComposerPendingPanel
          open={isComposerOpen}
          mode={composerMode}
          draftText={draftText}
          onOpenChange={handleComposerOpenChange}
          onDraftTextChange={
            composerMode === "create" ? handleDraftTextChange : undefined
          }
        />
      ) : null}

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
          composerMode === "create" ? handleDraftTextChange : undefined
        }
        onDraftClear={composerMode === "create" ? clearDraftText : undefined}
      />
    </div>
  );
}
