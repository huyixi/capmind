"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  MEMO_IMAGES_BUCKET,
  MEMO_IMAGE_URL_TTL_SECONDS,
} from "@/lib/memo-constants";
import { createSignedImageUrls } from "@/lib/supabase/storage";
import { createClient } from "@/lib/supabase/client";
import { Memo } from "@/lib/types";
import { areImagesEqual, isLocalImageUrl, isRemoteImageUrl } from "./image-utils";

interface UseMemoImagesArgs {
  memo: Memo;
  memoImages: string[];
  memoImageCount: number;
}

export function useMemoImages({
  memo,
  memoImages,
  memoImageCount,
}: UseMemoImagesArgs) {
  const hasDeferredImages = memoImages.length === 0 && memoImageCount > 0;
  const [displayImages, setDisplayImages] = useState<string[]>(memoImages);
  const [needsImageFetch, setNeedsImageFetch] = useState(hasDeferredImages);
  const [isResolvingImages, setIsResolvingImages] = useState(false);
  const imageAreaRef = useRef<HTMLDivElement | null>(null);
  const prevImagesRef = useRef<string[]>(memoImages);
  const resolvedImageKeyRef = useRef<string | null>(null);
  const signedImagesCacheRef = useRef<
    Map<string, { urls: string[]; expiresAt: number }>
  >(new Map());
  const isMountedRef = useRef(true);

  useEffect(() => {
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const getSignedImageUrls = useCallback(async (): Promise<string[]> => {
    const cacheKey = `${memo.id}:${memo.version}`;
    const cached = signedImagesCacheRef.current.get(cacheKey);
    const now = Date.now();
    if (cached && cached.expiresAt > now) {
      return cached.urls;
    }

    const supabase = createClient();
    const { data, error } = await supabase
      .from("memo_images")
      .select("url, sort_order")
      .eq("memo_id", memo.id)
      .order("sort_order", { ascending: true });

    if (error) {
      console.error("Error fetching memo images:", error);
      return [];
    }

    const rawImageUrls = (data ?? []).map((row: { url: string }) => row.url);
    if (rawImageUrls.length === 0) {
      signedImagesCacheRef.current.set(cacheKey, {
        urls: [],
        expiresAt: now + MEMO_IMAGE_URL_TTL_SECONDS * 1000,
      });
      return [];
    }

    const resolved = await createSignedImageUrls(
      supabase,
      MEMO_IMAGES_BUCKET,
      rawImageUrls,
      MEMO_IMAGE_URL_TTL_SECONDS,
    );

    signedImagesCacheRef.current.set(cacheKey, {
      urls: resolved,
      expiresAt: now + MEMO_IMAGE_URL_TTL_SECONDS * 1000,
    });

    return resolved;
  }, [memo.id, memo.version]);

  useEffect(() => {
    const previousImages = prevImagesRef.current;
    const memoKey = `${memo.id}:${memo.version}`;
    const hasDeferred = memoImages.length === 0 && memoImageCount > 0;

    if (hasDeferred) {
      if (displayImages.length > 0) {
        resolvedImageKeyRef.current = memoKey;
        setNeedsImageFetch(false);
        prevImagesRef.current = memoImages;
        return;
      }
      if (resolvedImageKeyRef.current !== memoKey) {
        if (displayImages.length > 0) {
          setDisplayImages([]);
        }
        setNeedsImageFetch(true);
      } else {
        setNeedsImageFetch(false);
      }
      prevImagesRef.current = memoImages;
      return;
    }

    setNeedsImageFetch(false);
    resolvedImageKeyRef.current = memoImages.length > 0 ? memoKey : null;

    if (areImagesEqual(previousImages, memoImages)) return;

    let cancelled = false;
    const previousIsLocal =
      previousImages.length > 0 && previousImages.every(isLocalImageUrl);
    const nextIsRemote =
      memoImages.length > 0 && memoImages.every(isRemoteImageUrl);

    if (
      previousIsLocal &&
      nextIsRemote &&
      previousImages.length === memoImages.length
    ) {
      setDisplayImages(previousImages);
      memoImages.forEach((url, index) => {
        const image = new window.Image();
        image.src = url;
        image.onload = () => {
          if (cancelled) return;
          setDisplayImages((current) => {
            if (current[index] !== previousImages[index]) return current;
            const next = [...current];
            next[index] = url;
            return next;
          });
        };
      });
    } else {
      setDisplayImages(memoImages);
    }

    prevImagesRef.current = memoImages;
    return () => {
      cancelled = true;
    };
  }, [displayImages.length, memo.id, memo.version, memoImageCount, memoImages]);

  const resolvePendingImages = useCallback(async () => {
    if (!needsImageFetch || isResolvingImages) return;
    setIsResolvingImages(true);
    try {
      const resolved = await getSignedImageUrls();
      if (!isMountedRef.current) return;
      if (resolved.length === 0) {
        resolvedImageKeyRef.current = `${memo.id}:${memo.version}`;
        setNeedsImageFetch(false);
        return;
      }
      setDisplayImages(resolved);
      resolvedImageKeyRef.current = `${memo.id}:${memo.version}`;
      setNeedsImageFetch(false);
    } catch (error) {
      console.error("Error resolving memo images:", error);
    } finally {
      if (isMountedRef.current) {
        setIsResolvingImages(false);
      }
    }
  }, [
    getSignedImageUrls,
    isResolvingImages,
    memo.id,
    memo.version,
    needsImageFetch,
  ]);

  useEffect(() => {
    if (!needsImageFetch) return;
    const target = imageAreaRef.current;
    if (!target) return;
    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (entry?.isIntersecting) {
          resolvePendingImages();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(target);
    return () => observer.disconnect();
  }, [needsImageFetch, resolvePendingImages]);

  return {
    displayImages,
    needsImageFetch,
    isResolvingImages,
    imageAreaRef,
    resolvePendingImages,
    getSignedImageUrls,
  };
}
