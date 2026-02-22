import { History, X } from "lucide-react";

interface SearchRecentListProps {
  items: string[];
  onSelect: (value: string) => void;
  onRemove: (value: string) => void;
}

export function SearchRecentList({
  items,
  onSelect,
  onRemove,
}: SearchRecentListProps) {
  return (
    <>
      {items.length === 0 ? (
        <div className="py-8 text-center text-sm text-muted-foreground">
          No recent searches yet.
        </div>
      ) : (
        <>
          <div className="text-xs text-muted-foreground mb-2 px-4">
            Recent searches
          </div>
          <div className="space-y-1 px-2">
            {items.map((item) => (
              <div
                key={item}
                className="flex w-full items-center gap-3 rounded-lg px-2 py-2 text-sm text-muted-foreground transition-colors hover:bg-muted/70"
              >
                <button
                  type="button"
                  onClick={() => onSelect(item)}
                  className="flex flex-1 items-center gap-3 text-left"
                >
                  <History className="size-4" />
                  <span className="text-foreground">{item}</span>
                </button>
                <button
                  type="button"
                  className="text-muted-foreground transition-colors hover:text-foreground"
                  onClick={() => onRemove(item)}
                  aria-label={`Remove ${item}`}
                >
                  <X className="size-4" />
                </button>
              </div>
            ))}
          </div>
        </>
      )}
    </>
  );
}
