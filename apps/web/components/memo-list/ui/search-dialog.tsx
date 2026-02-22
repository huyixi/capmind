"use client";

import dynamic from "next/dynamic";

const SearchDialog = dynamic(
  () => import("@/components/search-dialog").then((mod) => mod.SearchDialog),
  { ssr: false },
);

export const preloadMemoListSearchDialog = () => {
  void import("@/components/search-dialog");
};

interface MemoListSearchDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appliedQuery: string;
  onApplyQuery: (value: string) => void;
}

export function MemoListSearchDialog(props: MemoListSearchDialogProps) {
  return <SearchDialog {...props} />;
}
