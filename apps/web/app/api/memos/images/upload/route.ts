import { NextRequest, NextResponse } from "next/server";
import { createServerClient } from "@supabase/ssr";
import { MEMO_IMAGES_BUCKET } from "@/lib/memo-constants";

const FILES_FIELD = "files";

function getFileExtension(fileName: string) {
  const extension = fileName.split(".").pop()?.trim().toLowerCase();
  if (!extension) return "bin";
  return extension.replace(/[^a-z0-9]/g, "") || "bin";
}

function createStoragePath(userId: string, fileName: string) {
  const uniqueId =
    typeof crypto !== "undefined" && crypto.randomUUID
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2);
  return `${userId}/${Date.now()}-${uniqueId}.${getFileExtension(fileName)}`;
}

export async function POST(request: NextRequest) {
  const response = NextResponse.next();
  const supabase = createServerClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!,
    {
      cookies: {
        getAll() {
          return request.cookies.getAll();
        },
        setAll(cookiesToSet) {
          cookiesToSet.forEach(({ name, value, options }) =>
            response.cookies.set(name, value, options),
          );
        },
      },
    },
  );

  const {
    data: { user },
    error: userError,
  } = await supabase.auth.getUser();

  if (userError || !user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const formData = await request.formData();
  const files = formData
    .getAll(FILES_FIELD)
    .filter((value): value is File => value instanceof File);

  if (files.length === 0) {
    return NextResponse.json(
      { error: "No image files provided." },
      { status: 400 },
    );
  }

  if (files.some((file) => !file.type.startsWith("image/"))) {
    return NextResponse.json(
      { error: "Only image files are supported." },
      { status: 400 },
    );
  }

  const uploadedPaths: string[] = [];

  try {
    for (const file of files) {
      const path = createStoragePath(user.id, file.name);
      const { error } = await supabase.storage
        .from(MEMO_IMAGES_BUCKET)
        .upload(path, file);

      if (error) {
        throw error;
      }

      uploadedPaths.push(path);
    }

    const json = NextResponse.json({ paths: uploadedPaths });
    response.cookies.getAll().forEach((cookie) => {
      json.cookies.set(cookie);
    });
    return json;
  } catch (error) {
    console.error("Error uploading memo images:", error);

    if (uploadedPaths.length > 0) {
      const { error: cleanupError } = await supabase.storage
        .from(MEMO_IMAGES_BUCKET)
        .remove(uploadedPaths);
      if (cleanupError) {
        console.error("Error cleaning up memo images after upload failure:", cleanupError);
      }
    }

    return NextResponse.json(
      { error: "Failed to upload images. Please try again." },
      { status: 500 },
    );
  }
}
