"use client";

import type { ReactNode } from "react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";

const URL_REGEX = /https?:\/\/[^\s]+/g;
const TRAILING_PUNCTUATION = /[)\]\}.,;:!?]/;
const MAX_VISIBLE_LINK_LENGTH = 88;

const formatMemoLinkLabel = (url: string) => {
  if (url.length <= MAX_VISIBLE_LINK_LENGTH) return url;

  const queryIndex = url.indexOf("?");
  if (queryIndex < 0) return url;

  const hashIndex = url.indexOf("#", queryIndex);
  const baseUrl = url.slice(0, queryIndex);
  const hasHash = hashIndex >= 0;

  return `${baseUrl}?...${hasHash ? "#..." : ""}`;
};

export const getSupportedClipboardImageMimeTypes = () => {
  if (typeof ClipboardItem !== "undefined") {
    if (typeof ClipboardItem.supports === "function") {
      const supported: string[] = [];
      if (ClipboardItem.supports("image/webp")) supported.push("image/webp");
      if (ClipboardItem.supports("image/png")) supported.push("image/png");
      if (supported.length > 0) return supported;
    }
  }
  return ["image/png"];
};

export const copyTextToClipboard = async (
  content: string,
): Promise<boolean> => {
  if (!content) return false;
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(content);
      return true;
    }
    const textarea = document.createElement("textarea");
    textarea.value = content;
    textarea.setAttribute("readonly", "true");
    textarea.style.position = "fixed";
    textarea.style.top = "0";
    textarea.style.left = "0";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.focus();
    textarea.select();
    const didCopy = document.execCommand("copy");
    document.body.removeChild(textarea);
    return didCopy;
  } catch (error) {
    console.error("Failed to copy text:", error);
    return false;
  }
};

export const linkifyMemoText = (text: string) => {
  if (!text) return text;
  const nodes: ReactNode[] = [];
  let lastIndex = 0;
  let matchIndex = 0;
  const matches = text.matchAll(URL_REGEX);

  for (const match of matches) {
    const rawUrl = match[0];
    const start = match.index ?? 0;
    const end = start + rawUrl.length;

    if (start > lastIndex) {
      nodes.push(text.slice(lastIndex, start));
    }

    let trimmedUrl = rawUrl;
    let trailing = "";
    while (
      trimmedUrl.length > 0 &&
      TRAILING_PUNCTUATION.test(trimmedUrl[trimmedUrl.length - 1])
    ) {
      trailing = `${trimmedUrl[trimmedUrl.length - 1]}${trailing}`;
      trimmedUrl = trimmedUrl.slice(0, -1);
    }

    if (!trimmedUrl) {
      nodes.push(rawUrl);
      lastIndex = end;
      continue;
    }

    const linkKey = `${start}-${matchIndex}`;
    const linkLabel = formatMemoLinkLabel(trimmedUrl);
    nodes.push(
      <ContextMenu key={`link-${linkKey}`}>
        <ContextMenuTrigger asChild>
          <a
            href={trimmedUrl}
            target="_blank"
            rel="noreferrer noopener"
            title={trimmedUrl}
            className="break-all"
          >
            {linkLabel}
          </a>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuItem
            onSelect={(event) => {
              event.preventDefault();
              void copyTextToClipboard(trimmedUrl);
            }}
          >
            Copy link
          </ContextMenuItem>
          <ContextMenuItem
            onSelect={(event) => {
              event.preventDefault();
              window.open(trimmedUrl, "_blank", "noopener,noreferrer");
            }}
          >
            Open link
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>,
    );

    if (trailing) {
      nodes.push(trailing);
    }

    lastIndex = end;
    matchIndex += 1;
  }

  if (lastIndex < text.length) {
    nodes.push(text.slice(lastIndex));
  }

  return nodes;
};

export const convertBlobToImageType = async (
  blob: Blob,
  mimeType: string,
): Promise<Blob> => {
  if (blob.type === mimeType) return blob;

  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("Canvas not supported");
  }

  if (typeof createImageBitmap === "function") {
    const bitmap = await createImageBitmap(blob);
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    context.drawImage(bitmap, 0, 0);
    bitmap.close?.();
  } else {
    const image = await new Promise<HTMLImageElement>((resolve, reject) => {
      const url = URL.createObjectURL(blob);
      const img = new Image();
      img.onload = () => {
        URL.revokeObjectURL(url);
        resolve(img);
      };
      img.onerror = () => {
        URL.revokeObjectURL(url);
        reject(new Error("Image load failed"));
      };
      img.src = url;
    });
    canvas.width = image.naturalWidth || image.width;
    canvas.height = image.naturalHeight || image.height;
    context.drawImage(image, 0, 0);
  }

  const convertedBlob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((result) => {
      if (result) resolve(result);
      else reject(new Error("Image conversion failed"));
    }, mimeType);
  });

  return convertedBlob;
};

export const prepareClipboardImageBlobs = async (
  blob: Blob,
  mimeTypes: string[],
): Promise<Record<string, Blob>> => {
  const prepared: Record<string, Blob> = {};

  for (const mimeType of mimeTypes) {
    if (prepared[mimeType]) continue;
    if (blob.type === mimeType) {
      prepared[mimeType] = blob;
      continue;
    }
    try {
      const converted = await convertBlobToImageType(blob, mimeType);
      prepared[mimeType] = converted;
    } catch (error) {
      console.error(`Failed to convert image to ${mimeType}:`, error);
    }
  }

  if (!prepared["image/png"]) {
    try {
      prepared["image/png"] = await convertBlobToImageType(blob, "image/png");
    } catch (error) {
      console.error("Failed to convert image to PNG:", error);
      throw error;
    }
  }

  return prepared;
};
