"use client";

import { useEffect, useState } from "react";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import Image from "next/image";
import { X } from "lucide-react";
import { isLocalImageUrl } from "../logic/image-utils";

interface MemoImagePreviewProps {
  imageUrls: string[];
  initialIndex: number;
  onClose: () => void;
}

const PLACEHOLDER_SRC = "/placeholder.svg";
const SUPABASE_STORAGE_PREFIX = "/storage/v1/object/";

const isRelativeImageUrl = (url: string) => url.startsWith("/");
const isHttpImageUrl = (url: string) =>
  url.startsWith("http://") || url.startsWith("https://");

let supabaseEnvHost: string | null = null;
let supabaseEnvProtocol: string | null = null;
let supabaseEnvPort: string | null = null;

if (process.env.NEXT_PUBLIC_SUPABASE_URL) {
  try {
    const supabaseEnvUrl = new URL(process.env.NEXT_PUBLIC_SUPABASE_URL);
    supabaseEnvHost = supabaseEnvUrl.hostname;
    supabaseEnvProtocol = supabaseEnvUrl.protocol;
    supabaseEnvPort = supabaseEnvUrl.port || null;
  } catch {
    // Ignore invalid environment URL.
  }
}

const isSupabaseStoragePath = (pathname: string) =>
  pathname.startsWith(SUPABASE_STORAGE_PREFIX);

const isHostedSupabase = (hostname: string) =>
  hostname === "supabase.co" ||
  hostname.endsWith(".supabase.co") ||
  hostname.endsWith(".supabase.in");

const isAllowedRemoteImageUrl = (url: string) => {
  try {
    const parsedUrl = new URL(url);
    if (!isSupabaseStoragePath(parsedUrl.pathname)) return false;

    if (supabaseEnvHost && parsedUrl.hostname === supabaseEnvHost) {
      if (supabaseEnvProtocol && parsedUrl.protocol !== supabaseEnvProtocol) {
        return false;
      }
      if (supabaseEnvPort && parsedUrl.port !== supabaseEnvPort) {
        return false;
      }
      return true;
    }

    if (parsedUrl.protocol !== "https:") return false;
    return isHostedSupabase(parsedUrl.hostname);
  } catch {
    return false;
  }
};

const getSafeImageSrc = (url: string) => {
  if (typeof url !== "string") return PLACEHOLDER_SRC;
  const trimmed = url.trim();
  return trimmed.length > 0 ? trimmed : PLACEHOLDER_SRC;
};

const shouldUseNextImage = (src: string) => {
  if (isLocalImageUrl(src)) return false;
  if (isRelativeImageUrl(src)) return true;
  if (!isHttpImageUrl(src)) return false;
  return isAllowedRemoteImageUrl(src);
};

const resolveImage = (url: string) => {
  const src = getSafeImageSrc(url);
  return { src, useNextImage: shouldUseNextImage(src) };
};

const clampIndex = (index: number, length: number) => {
  if (!Number.isFinite(index)) return 0;
  if (length <= 0) return 0;
  if (index < 0) return 0;
  if (index >= length) return length - 1;
  return index;
};

export function MemoImagePreview({
  imageUrls,
  initialIndex,
  onClose,
}: MemoImagePreviewProps) {
  const safeImageUrls = Array.isArray(imageUrls) ? imageUrls : [];
  const [activeIndex, setActiveIndex] = useState(() =>
    clampIndex(initialIndex, safeImageUrls.length),
  );

  useEffect(() => {
    setActiveIndex((current) => {
      if (safeImageUrls.length === 0) return 0;
      const hasValidInitialIndex =
        Number.isFinite(initialIndex) &&
        initialIndex >= 0 &&
        initialIndex < safeImageUrls.length;
      if (hasValidInitialIndex && initialIndex !== current) {
        return initialIndex;
      }
      if (!Number.isFinite(current) || current < 0) return 0;
      if (current >= safeImageUrls.length) return safeImageUrls.length - 1;
      return current;
    });
  }, [safeImageUrls.length, initialIndex]);

  const activeUrl = safeImageUrls[activeIndex] ?? "";
  const activeImage = resolveImage(activeUrl);
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent
        showCloseButton={false}
        className="!fixed !inset-0 !h-[100dvh] !w-[100dvw] !max-w-none !max-h-none !translate-x-0 !translate-y-0 rounded-none border-none bg-black p-0 shadow-none"
      >
        <DialogTitle className="sr-only">Image preview</DialogTitle>
        <button
          type="button"
          onClick={onClose}
          className="absolute left-4 top-[calc(env(safe-area-inset-top)+1rem)] z-10 inline-flex h-9 w-9 items-center justify-center rounded-full bg-white/30 text-white transition hover:bg-white/50"
          aria-label="Close image preview"
        >
          <X className="h-4 w-4" />
        </button>
        <div className="flex h-full w-full flex-col">
          <div className="flex min-h-0 flex-1 items-center justify-center p-4">
            {activeImage.useNextImage ? (
              <div className="relative h-full w-full max-h-full max-w-full">
                <Image
                  src={activeImage.src}
                  alt="Expanded memo image"
                  fill
                  sizes="(max-width: 768px) 100vw, 1200px"
                  className="object-contain"
                />
              </div>
            ) : (
              <>
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={activeImage.src}
                  alt="Expanded memo image"
                  width={1200}
                  height={1200}
                  className="h-full w-full object-contain"
                />
              </>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
