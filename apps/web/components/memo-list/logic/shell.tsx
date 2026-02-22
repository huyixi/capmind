"use client";

import dynamic from "next/dynamic";
import type { Memo } from "@/lib/types";
import type { AuthUser as User } from "@supabase/supabase-js";
import type { MemoComposerActions, MemoSearchActions } from "./container";

const MemoListContainer = dynamic(
  () =>
    import("@/components/memo-list/logic/container").then(
      (mod) => mod.MemoListContainer,
    ),
  { ssr: false },
);

interface MemoListShellProps {
  initialUser: User | null;
  initialMemos?: Memo[];
  onEdit: (memo: Memo) => void;
  onRegisterComposerActions: (actions: MemoComposerActions | null) => void;
  onRegisterSearchActions: (actions: MemoSearchActions | null) => void;
  onResetComposer: () => void;
}

export function MemoListShell({
  initialUser,
  initialMemos,
  onEdit,
  onRegisterComposerActions,
  onRegisterSearchActions,
  onResetComposer,
}: MemoListShellProps) {
  return (
    <MemoListContainer
      initialUser={initialUser}
      initialMemos={initialMemos}
      onEdit={onEdit}
      onRegisterComposerActions={onRegisterComposerActions}
      onRegisterSearchActions={onRegisterSearchActions}
      onResetComposer={onResetComposer}
    />
  );
}
