"use client";

import { useEffect, useRef, useState } from "react";
import { Search, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { SearchRecentList } from "@/components/search-recent-list";
import { useRecentSearches } from "@/hooks/use-recent-searches";

interface SearchDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appliedQuery: string;
  onApplyQuery: (value: string) => void;
}

export function SearchDialog({
  open,
  onOpenChange,
  appliedQuery,
  onApplyQuery,
}: SearchDialogProps) {
  const [draftQuery, setDraftQuery] = useState(appliedQuery);
  const {
    recentSearches,
    addRecentSearch,
    removeRecentSearch,
    reloadRecentSearches,
  } = useRecentSearches();
  const inputRef = useRef<HTMLInputElement | null>(null);
  const trimmedQuery = draftQuery.trim();

  const applyQueryAndClose = (value: string) => {
    const trimmed = value.trim();
    onApplyQuery(trimmed);
    if (trimmed) {
      addRecentSearch(trimmed);
    }
    onOpenChange(false);
  };

  useEffect(() => {
    if (!open) return;
    setDraftQuery(appliedQuery);
    reloadRecentSearches();
    const id = window.setTimeout(() => {
      inputRef.current?.focus();
    }, 0);
    return () => window.clearTimeout(id);
  }, [appliedQuery, open, reloadRecentSearches]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        showCloseButton={false}
        className="inset-0 h-full w-full max-w-none translate-x-0 translate-y-0 overflow-hidden rounded-none border-none bg-white p-0 sm:top-[50%] sm:left-[50%] sm:right-auto sm:bottom-auto sm:h-auto sm:max-h-[85vh] sm:w-full sm:max-w-lg sm:translate-x-[-50%] sm:translate-y-[-50%] sm:rounded-xl sm:border sm:border-border/40 sm:bg-background sm:shadow-lg"
      >
        <DialogTitle className="sr-only">Search memos</DialogTitle>
        <div className="flex h-full flex-col bg-white">
          <div className="flex items-center gap-3 border-b px-4 py-1">
            <Search className="size-4 text-muted-foreground" />
            <div className="relative flex-1 min-w-0">
              <Input
                ref={inputRef}
                type="text"
                value={draftQuery}
                onChange={(event) => setDraftQuery(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    applyQueryAndClose(draftQuery);
                  }
                }}
                placeholder="Search memos"
                className="h-10 flex-1 border-none bg-transparent pl-0 pr-9 text-base shadow-none focus-visible:ring-0 [&::-webkit-search-cancel-button]:appearance-none [&::-webkit-search-decoration]:appearance-none"
              />
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={() => onOpenChange(false)}
              aria-label="Close search"
              className="h-8 w-8 shrink-0 text-muted-foreground hover:text-foreground"
            >
              <X className="size-6" />
            </Button>
          </div>

          <div className="flex-1 overflow-y-auto pb-2">
            <div className="pt-2">
              {trimmedQuery ? (
                <button
                  type="button"
                  onClick={() => applyQueryAndClose(trimmedQuery)}
                  className="w-full px-3 py-3 text-left transition-colors rounded-md hover:bg-muted/40"
                >
                  <div className="flex items-center gap-3">
                    <div className="inline-flex size-8 items-center justify-center rounded-sm bg-muted text-muted-foreground">
                      <Search className="size-4" />
                    </div>
                    <div className="text-base text-foreground">
                      {trimmedQuery}
                    </div>
                  </div>
                </button>
              ) : (
                <SearchRecentList
                  items={recentSearches}
                  onSelect={applyQueryAndClose}
                  onRemove={removeRecentSearch}
                />
              )}
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
