"use client";

import { Suspense, lazy } from "react";
import { type AuthUser as User } from "@supabase/supabase-js";
import { SearchIcon, X } from "lucide-react";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";

const UserMenu = lazy(() =>
  import("@/components/user-menu").then((mod) => ({
    default: mod.UserMenu,
  })),
);

interface HeaderProps {
  user: User | null;
  onRefresh: () => void;
  onToggleTrash: () => void;
  isTrashActive: boolean;
  onSearchOpen: () => void;
  onClearSearch: () => void;
  isRefreshing: boolean;
  isSyncing: boolean;
  searchQuery: string;
}

function UserMenuFallback({ user }: { user: User | null }) {
  const displayName =
    user?.user_metadata?.display_name ||
    user?.user_metadata?.username ||
    user?.email?.split("@")[0] ||
    "User";

  return (
    <button
      type="button"
      className="inline-flex h-9 items-center px-1 text-sm"
      disabled
      aria-hidden="true"
    >
      <span className="font-medium">{displayName}</span>
    </button>
  );
}

export function Header({
  user,
  onRefresh,
  onToggleTrash,
  isTrashActive,
  onSearchOpen,
  onClearSearch,
  isRefreshing,
  isSyncing,
  searchQuery,
}: HeaderProps) {
  const shouldShowClear = Boolean(searchQuery.trim());

  return (
    <header className="sticky top-0 z-10 max-w-xl backdrop-blur-md bg-[#F3F1F0] w-full mx-auto border-x animate-fade-in origin-center border-b border-tertiary  flex flex-row items-center justify-center px-1 animate-fade-in">
      <div className="flex h-12 w-full items-center justify-between pe-2">
        <Suspense fallback={<UserMenuFallback user={user} />}>
          <UserMenu
            user={user}
            onRefresh={onRefresh}
            isRefreshing={isRefreshing}
            isSyncing={isSyncing}
            onToggleTrash={onToggleTrash}
            isTrashActive={isTrashActive}
          />
        </Suspense>

        <div className="flex items-center gap-2 ml-auto">
          <InputGroup className="w-56 h-7 rounded-sm border-tertiary bg-[#FAF8F7] shadow-none">
            <InputGroupAddon align="inline-start">
              <SearchIcon className="text-muted-foreground" />
            </InputGroupAddon>
            <InputGroupInput
              id="header-search"
              type="text"
              readOnly
              placeholder="Search..."
              className="h-7 text-[14px] leading-7 py-0 placeholder:text-[14px]"
              value={searchQuery}
              onClick={onSearchOpen}
              onFocus={onSearchOpen}
            />
            {shouldShowClear ? (
              <InputGroupAddon align="inline-end">
                <InputGroupButton
                  size="icon-xs"
                  aria-label="Clear search"
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={(event) => {
                    event.stopPropagation();
                    onClearSearch();
                  }}
                >
                  <X className="h-4 w-4" />
                </InputGroupButton>
              </InputGroupAddon>
            ) : null}
          </InputGroup>
        </div>
      </div>
    </header>
  );
}
