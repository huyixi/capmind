import type { SupabaseClient } from "@supabase/supabase-js";

const STORAGE_OBJECT_PREFIX = "/storage/v1/object/";

function stripLeadingSlash(value: string) {
  return value.replace(/^\/+/, "");
}

export function extractStoragePath(raw: string, bucket: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  if (trimmed.startsWith("data:") || trimmed.startsWith("blob:")) return null;

  if (!trimmed.includes("://")) {
    const cleaned = stripLeadingSlash(trimmed);
    const bucketPrefix = `${bucket}/`;
    const publicPrefix = `public/${bucket}/`;
    const signPrefix = `sign/${bucket}/`;

    if (cleaned.startsWith(bucketPrefix)) {
      return cleaned.slice(bucketPrefix.length) || null;
    }
    if (cleaned.startsWith(publicPrefix)) {
      return cleaned.slice(publicPrefix.length) || null;
    }
    if (cleaned.startsWith(signPrefix)) {
      return cleaned.slice(signPrefix.length) || null;
    }

    return cleaned || null;
  }

  try {
    const url = new URL(trimmed);
    const prefixIndex = url.pathname.indexOf(STORAGE_OBJECT_PREFIX);
    if (prefixIndex === -1) return null;
    const pathAfterPrefix = url.pathname.slice(
      prefixIndex + STORAGE_OBJECT_PREFIX.length,
    );
    const segments = pathAfterPrefix.split("/").filter(Boolean);
    if (segments.length < 3) return null;
    const bucketFromUrl = segments[1];
    if (bucketFromUrl !== bucket) return null;
    const objectPath = segments.slice(2).join("/");
    return objectPath || null;
  } catch {
    return null;
  }
}

export async function createSignedImageUrls(
  supabase: SupabaseClient,
  bucket: string,
  rawUrls: string[],
  expiresInSeconds: number,
): Promise<string[]> {
  if (rawUrls.length === 0) return [];

  const paths: string[] = [];
  const pathByIndex = new Map<number, string>();

  rawUrls.forEach((raw, index) => {
    const path = extractStoragePath(raw, bucket);
    if (!path) return;
    paths.push(path);
    pathByIndex.set(index, path);
  });

  if (paths.length === 0) {
    return rawUrls;
  }

  const { data, error } = await supabase.storage
    .from(bucket)
    .createSignedUrls(paths, expiresInSeconds);

  if (error) {
    console.error("Error creating signed image URLs:", error);
    return rawUrls.map((raw, index) => {
      const path = pathByIndex.get(index);
      if (!path) return raw;
      return supabase.storage.from(bucket).getPublicUrl(path).data.publicUrl;
    });
  }

  const signedByPath = new Map<string, string>();
  data?.forEach((item) => {
    if (item?.path && item?.signedUrl) {
      signedByPath.set(item.path, item.signedUrl);
    }
  });

  return rawUrls.map((raw, index) => {
    const path = pathByIndex.get(index);
    if (!path) return raw;
    return (
      signedByPath.get(path) ??
      supabase.storage.from(bucket).getPublicUrl(path).data.publicUrl
    );
  });
}
