"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldContent,
  FieldDescription as FieldDescriptionText,
  FieldError,
  FieldLabel,
  FieldTitle,
} from "@/components/ui/field";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  ChevronDown,
  Download,
  List,
  LogOut,
  RotateCcw,
  Trash2,
  User,
} from "lucide-react";
import { createClient } from "@/lib/supabase/client";
import { useRouter } from "next/navigation";
import { type AuthUser } from "@supabase/supabase-js";
import { useMemoExport } from "@/hooks/use-memo-export";
import { type ExportRange } from "@/lib/memo-export";
import { Spinner } from "@/components/ui/spinner";

interface UserMenuProps {
  user: AuthUser | null;
  onRefresh: () => void;
  isRefreshing: boolean;
  isSyncing: boolean;
  onToggleTrash: () => void;
  isTrashActive: boolean;
}

export function UserMenu({
  user,
  onRefresh,
  isRefreshing,
  isSyncing,
  onToggleTrash,
  isTrashActive,
}: UserMenuProps) {
  const [hasMounted, setHasMounted] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  const [isNameDialogOpen, setIsNameDialogOpen] = useState(false);
  const [isExportDialogOpen, setIsExportDialogOpen] = useState(false);
  const [selectedRange, setSelectedRange] = useState<ExportRange>("day");
  const [currentDisplayName, setCurrentDisplayName] = useState("User");
  const [nameInput, setNameInput] = useState("");
  const [nameError, setNameError] = useState<string | null>(null);
  const [isSavingName, setIsSavingName] = useState(false);
  const { exportError, isExporting, exportMemos, copyMemos, clearExportError } =
    useMemoExport();
  const router = useRouter();
  const supabase = createClient();

  const displayName =
    user?.user_metadata?.display_name ||
    user?.user_metadata?.username ||
    user?.email?.split("@")[0] ||
    "User";

  useEffect(() => {
    setHasMounted(true);
  }, []);

  useEffect(() => {
    setCurrentDisplayName(displayName);
    setNameInput(displayName);
  }, [displayName]);

  const handleSignOut = async () => {
    await supabase.auth.signOut();
    router.push("/login");
    router.refresh();
  };

  const handleSaveName = async () => {
    const trimmed = nameInput.trim();
    if (!trimmed) {
      setNameError("Username cannot be empty.");
      return;
    }

    setIsSavingName(true);
    setNameError(null);

    const { error } = await supabase.auth.updateUser({
      data: { display_name: trimmed },
    });

    if (error) {
      setNameError(error.message);
      setIsSavingName(false);
      return;
    }

    setCurrentDisplayName(trimmed);
    setNameInput(trimmed);
    setIsSavingName(false);
    router.refresh();
    setIsNameDialogOpen(false);
  };

  const handleRefresh = () => {
    setIsOpen(false);
    onRefresh();
  };

  const handleToggleTrash = () => {
    setIsOpen(false);
    onToggleTrash();
  };

  const trashLabel = isTrashActive ? "All" : "Trash";
  const TrashIcon = isTrashActive ? List : Trash2;

  const handleExport = async () => {
    const success = await exportMemos(selectedRange);
    if (success) {
      setIsExportDialogOpen(false);
    }
  };

  const handleCopy = async () => {
    const success = await copyMemos(selectedRange);
    if (success) {
      setIsExportDialogOpen(false);
    }
  };

  const exportChoices = [
    { range: "day" as const, title: "Today", description: "Only today" },
    { range: "week" as const, title: "This week", description: "Last 7 days" },
    { range: "all" as const, title: "All", description: "All memos" },
  ];

  if (!user) {
    return (
      <Button
        variant="ghost"
        size="sm"
        type="button"
        onClick={() => router.push("/login")}
      >
        Sign in
      </Button>
    );
  }

  if (!hasMounted) {
    return (
      <Button variant="ghost" size="sm" type="button" className="px-0" disabled>
        <span className="font-medium">{currentDisplayName}</span>
        {isSyncing || isRefreshing ? (
          <Spinner className="ml-2 size-3.5 text-muted-foreground" />
        ) : null}
      </Button>
    );
  }

  return (
    <>
      <DropdownMenu
        open={isOpen}
        onOpenChange={(nextOpen) => {
          setIsOpen(nextOpen);
          if (!nextOpen) {
            clearExportError();
          }
        }}
      >
        <DropdownMenuTrigger className="p-0" asChild>
          <Button variant="ghost" size="sm" type="button" className="px-1!">
            <span className="font-medium">{currentDisplayName}</span>
            {isSyncing || isRefreshing ? (
              <Spinner className="size-3.5 text-muted-foreground" />
            ) : (
              <ChevronDown />
            )}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem onSelect={handleRefresh}>
            <RotateCcw className="h-4 w-4" />
            Refresh
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={handleToggleTrash}>
            <TrashIcon className="h-4 w-4" />
            {trashLabel}
          </DropdownMenuItem>
          <DropdownMenuItem
            onSelect={() => {
              clearExportError();
              setIsExportDialogOpen(true);
            }}
          >
            <Download className="h-4 w-4" />
            Export
          </DropdownMenuItem>
          <DropdownMenuItem
            onSelect={() => {
              setNameInput(displayName);
              setNameError(null);
              setIsNameDialogOpen(true);
            }}
          >
            <User className="h-4 w-4" />
            Profile
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={handleSignOut}>
            <LogOut className="h-4 w-4" />
            Sign out
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <Dialog
        open={isNameDialogOpen}
        onOpenChange={(nextOpen) => {
          setIsNameDialogOpen(nextOpen);
          if (!nextOpen) {
            setNameError(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Profile</DialogTitle>
            <DialogDescription>
              Update the display name shown in your header.
            </DialogDescription>
          </DialogHeader>
          <Input
            value={nameInput}
            onChange={(e) => setNameInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleSaveName();
              }
            }}
            placeholder="Username"
          />
          {nameError ? <FieldError>{nameError}</FieldError> : null}
          <DialogFooter className="sm:justify-between sm:items-center">
            <Button
              variant="ghost"
              onClick={() => {
                setIsNameDialogOpen(false);
                setNameInput(displayName);
                setNameError(null);
              }}
              disabled={isSavingName}
            >
              Cancel
            </Button>
            <Button
              onClick={handleSaveName}
              disabled={isSavingName || !nameInput.trim()}
            >
              {isSavingName ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={isExportDialogOpen}
        onOpenChange={(nextOpen) => {
          setIsExportDialogOpen(nextOpen);
          if (!nextOpen) {
            clearExportError();
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Export</DialogTitle>
            <DialogDescription>
              Choose a range to copy or export.
            </DialogDescription>
          </DialogHeader>
          <RadioGroup
            value={selectedRange}
            onValueChange={(value) => {
              if (value) {
                setSelectedRange(value as ExportRange);
                clearExportError();
              }
            }}
          >
            {exportChoices.map((choice) => {
              const inputId = `export-range-${choice.range}`;
              return (
                <FieldLabel key={choice.range} htmlFor={inputId}>
                  <Field orientation="horizontal">
                    <FieldContent>
                      <FieldTitle>{choice.title}</FieldTitle>
                      <FieldDescriptionText>
                        {choice.description}
                      </FieldDescriptionText>
                    </FieldContent>
                    <RadioGroupItem value={choice.range} id={inputId} />
                  </Field>
                </FieldLabel>
              );
            })}
          </RadioGroup>
          {exportError ? (
            <DialogDescription>{exportError}</DialogDescription>
          ) : null}
          <DialogFooter className="sm:justify-between sm:items-center">
            <Button
              variant="ghost"
              onClick={handleExport}
              disabled={isExporting}
            >
              Export
            </Button>
            <Button size="lg" onClick={handleCopy} disabled={isExporting}>
              Copy
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
