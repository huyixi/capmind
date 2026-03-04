"use client";

import dynamic from "next/dynamic";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { type AuthUser as User } from "@supabase/supabase-js";
import { useOnlineStatus } from "@/hooks/use-online-status";
import { useMemoComposerController } from "@/hooks/use-memo-composer-controller";
import { useMemoShortcuts } from "@/hooks/use-memo-shortcuts";
import { MemoCreateButton } from "@/components/memo-create-button";
import { MemoComposerPanel } from "@/components/memo-composer/ui/panel";
import type { MemoSearchActions } from "@/components/memo-list/logic/container";
import { reportComposerPerfMetric } from "@/lib/composer-performance";
import type { Memo } from "@/lib/types";

const DRAFT_STORAGE_KEY = "memo-draft:create";
const LIST_IDLE_TIMEOUT_DEFAULT_MS = 2000;
const LIST_IDLE_TIMEOUT_MIN_MS = 800;
const LIST_IDLE_TIMEOUT_MAX_MS = 4000;
const SUBMIT_USER_CACHE_TTL_MS = 30_000;
const SUBMIT_USER_ERROR_CACHE_TTL_MS = 5_000;
const SUBMIT_PREWARM_IDLE_TIMEOUT_DEFAULT_MS = 1500;

type IdleCallbackWindow = Window & {
  requestIdleCallback?: (
    callback: () => void,
    options?: { timeout: number },
  ) => number;
  cancelIdleCallback?: (handle: number) => void;
};

type NetworkConnectionInfo = {
  effectiveType?: string;
  saveData?: boolean;
};

type NetworkNavigator = Navigator & {
  connection?: NetworkConnectionInfo;
  mozConnection?: NetworkConnectionInfo;
  webkitConnection?: NetworkConnectionInfo;
  deviceMemory?: number;
};

const clamp = (value: number, min: number, max: number) =>
  Math.max(min, Math.min(max, value));

function computeAdaptiveIdleTimeoutMs(baseTimeoutMs: number) {
  if (typeof navigator === "undefined") {
    return baseTimeoutMs;
  }

  let timeoutMs = baseTimeoutMs;
  const networkNavigator = navigator as NetworkNavigator;
  const connection =
    networkNavigator.connection ??
    networkNavigator.mozConnection ??
    networkNavigator.webkitConnection;

  if (connection?.saveData) {
    timeoutMs += 1200;
  }

  switch (connection?.effectiveType) {
    case "slow-2g":
    case "2g":
      timeoutMs += 1400;
      break;
    case "3g":
      timeoutMs += 700;
      break;
    case "4g":
      timeoutMs -= 200;
      break;
    default:
      break;
  }

  const hardwareConcurrency = navigator.hardwareConcurrency;
  if (Number.isFinite(hardwareConcurrency)) {
    if (hardwareConcurrency <= 2) {
      timeoutMs += 1000;
    } else if (hardwareConcurrency <= 4) {
      timeoutMs += 500;
    } else if (hardwareConcurrency >= 8) {
      timeoutMs -= 200;
    }
  }

  const deviceMemory = networkNavigator.deviceMemory;
  if (typeof deviceMemory === "number" && Number.isFinite(deviceMemory)) {
    if (deviceMemory <= 2) {
      timeoutMs += 900;
    } else if (deviceMemory <= 4) {
      timeoutMs += 450;
    } else if (deviceMemory >= 8) {
      timeoutMs -= 200;
    }
  }

  return clamp(timeoutMs, LIST_IDLE_TIMEOUT_MIN_MS, LIST_IDLE_TIMEOUT_MAX_MS);
}

const DeferredMemoListShell = dynamic(
  () =>
    import("@/components/memo-list/logic/shell").then(
      (mod) => mod.MemoListShell,
    ),
  {
    ssr: false,
    loading: () => null,
  },
);

type SubmitUserCacheEntry = {
  user: User | null;
  expiresAtMs: number;
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
  const [isBackgroundWorkEnabled, setIsBackgroundWorkEnabled] = useState(false);
  const isOnline = useOnlineStatus();
  const composerOpenAtRef = useRef<number | null>(null);
  const hasReportedComposerFocusRef = useRef(false);
  const hasReportedFirstKeystrokeRef = useRef(false);
  const submitUserCacheRef = useRef<SubmitUserCacheEntry | null>(null);
  const submitUserInFlightRef = useRef<Promise<User | null> | null>(null);
  const hasPrewarmedSubmitPathRef = useRef(false);
  const adaptiveIdleTimeoutMs = useMemo(
    () => computeAdaptiveIdleTimeoutMs(LIST_IDLE_TIMEOUT_DEFAULT_MS),
    [],
  );
  const adaptiveSubmitPrewarmTimeoutMs = useMemo(
    () =>
      clamp(
        Math.round(adaptiveIdleTimeoutMs * 0.75),
        700,
        SUBMIT_PREWARM_IDLE_TIMEOUT_DEFAULT_MS + 1200,
      ),
    [adaptiveIdleTimeoutMs],
  );

  const resolveSubmitUser = useCallback(async (): Promise<User | null> => {
    if (initialUser) return initialUser;

    const now = Date.now();
    const cached = submitUserCacheRef.current;
    if (cached && cached.expiresAtMs > now) {
      return cached.user;
    }
    if (submitUserInFlightRef.current) {
      return submitUserInFlightRef.current;
    }

    const request = (async () => {
      try {
        const { createClient } = await import("@/lib/supabase/client");
        const supabase = createClient();
        const { data, error } = await supabase.auth.getSession();
        const user = error || !data?.session?.user ? null : data.session.user;
        submitUserCacheRef.current = {
          user,
          expiresAtMs: Date.now() + SUBMIT_USER_CACHE_TTL_MS,
        };
        return user;
      } catch {
        submitUserCacheRef.current = {
          user: null,
          expiresAtMs: Date.now() + SUBMIT_USER_ERROR_CACHE_TTL_MS,
        };
        return null;
      }
    })();
    submitUserInFlightRef.current = request;
    try {
      return await request;
    } finally {
      submitUserInFlightRef.current = null;
    }
  }, [initialUser]);

  useEffect(() => {
    if (!initialUser) return;
    submitUserCacheRef.current = {
      user: initialUser,
      expiresAtMs: Date.now() + SUBMIT_USER_CACHE_TTL_MS,
    };
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

  const prewarmSubmitPath = useCallback(() => {
    if (hasPrewarmedSubmitPathRef.current) return;
    hasPrewarmedSubmitPathRef.current = true;

    const warm = () => {
      void resolveSubmitUser();

      if (isOnline) {
        void Promise.all([
          import("@/lib/supabase/client"),
          import("@/lib/memo-constants"),
        ]);
        return;
      }

      void Promise.all([
        import("@/lib/offline/memo-queue"),
        import("@/lib/offline/optimistic-images"),
        import("@/lib/memo-cache"),
      ]);
    };

    const idleWindow = window as IdleCallbackWindow;
    if (typeof idleWindow.requestIdleCallback === "function") {
      idleWindow.requestIdleCallback(
        () => {
          warm();
        },
        { timeout: adaptiveSubmitPrewarmTimeoutMs },
      );
      return;
    }

    window.setTimeout(warm, 0);
  }, [adaptiveSubmitPrewarmTimeoutMs, isOnline, resolveSubmitUser]);

  useEffect(() => {
    let cancelled = false;
    const enableBackgroundWork = () => {
      if (cancelled) return;
      setIsBackgroundWorkEnabled(true);
      setShouldMountMemoList(true);
    };
    const idleWindow = window as IdleCallbackWindow;
    if (typeof idleWindow.requestIdleCallback === "function") {
      const idleHandle = idleWindow.requestIdleCallback(
        () => {
          enableBackgroundWork();
        },
        { timeout: adaptiveIdleTimeoutMs },
      );
      return () => {
        cancelled = true;
        if (typeof idleWindow.cancelIdleCallback === "function") {
          idleWindow.cancelIdleCallback(idleHandle);
        }
      };
    }
    const timer = window.setTimeout(enableBackgroundWork, adaptiveIdleTimeoutMs);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [adaptiveIdleTimeoutMs]);

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
    prewarmSubmitPath();
    openCreateComposer();
  }, [openCreateComposer, prewarmSubmitPath]);

  const handleCreateHover = useCallback(() => {
    prewarmSubmitPath();
  }, [prewarmSubmitPath]);

  const handleEditOpen = useCallback(
    (memo: Memo) => {
      prewarmSubmitPath();
      enableBackgroundWorkNow();
      void handleEditOpenRaw(memo);
    },
    [enableBackgroundWorkNow, handleEditOpenRaw, prewarmSubmitPath],
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

  useEffect(() => {
    if (!isComposerOpen) return;
    prewarmSubmitPath();
  }, [isComposerOpen, prewarmSubmitPath]);

  return (
    <div className="min-h-screen flex flex-col bg-background">
      {shouldMountMemoList ? (
        <DeferredMemoListShell
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
        onPointerEnter={handleCreateHover}
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
