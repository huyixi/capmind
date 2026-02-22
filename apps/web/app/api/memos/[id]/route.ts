import { NextRequest, NextResponse } from "next/server";
import { createServerClient } from "@supabase/ssr";
import { buildMemoFromRow, MEMO_SELECT_FIELDS, type MemoRow } from "@/lib/memos";
import { MEMO_IMAGES_BUCKET } from "@/lib/memo-constants";
import { extractStoragePath } from "@/lib/supabase/storage";

function incrementNumericString(value: string) {
  const digits = value.split("");
  let carry = 1;

  for (let i = digits.length - 1; i >= 0 && carry > 0; i -= 1) {
    const next = (digits[i].charCodeAt(0) - 48) + carry;
    if (next >= 10) {
      digits[i] = "0";
      carry = 1;
    } else {
      digits[i] = String.fromCharCode(48 + next);
      carry = 0;
    }
  }

  if (carry > 0) {
    digits.unshift("1");
  }

  return digits.join("");
}

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const resolvedParams = await params;
  let body: { text?: string; expectedVersion?: string; imageUrls?: string[] };
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON" }, { status: 400 });
  }

  if (typeof body.text !== "string" || typeof body.expectedVersion !== "string") {
    return NextResponse.json({ error: "Invalid payload" }, { status: 400 });
  }
  if (
    body.imageUrls !== undefined &&
    (!Array.isArray(body.imageUrls) ||
      !body.imageUrls.every((item) => typeof item === "string"))
  ) {
    return NextResponse.json({ error: "Invalid payload" }, { status: 400 });
  }

  const memoId = resolvedParams?.id;
  const expectedVersion = body.expectedVersion;
  const trimmedText = body.text.trim();
  const imageUrls = Array.isArray(body.imageUrls) ? body.imageUrls : null;
  const shouldUpdateImages = imageUrls !== null;
  const normalizedImageUrls = imageUrls
    ? imageUrls
        .map(
          (raw) =>
            extractStoragePath(raw, MEMO_IMAGES_BUCKET) ?? raw.trim(),
        )
        .filter((value) => value.length > 0)
    : [];
  if (!/^\d+$/.test(expectedVersion)) {
    return NextResponse.json({ error: "Invalid version" }, { status: 400 });
  }

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

  const updatedAt = new Date().toISOString();
  const nextVersion = incrementNumericString(expectedVersion);
  const { data: updated, error: updateError } = await supabase
    .from("memos")
    .update({ text: trimmedText, updated_at: updatedAt, version: nextVersion })
    .eq("id", memoId)
    .eq("user_id", user.id)
    .eq("version", expectedVersion)
    .select(MEMO_SELECT_FIELDS)
    .order("sort_order", { referencedTable: "memo_images", ascending: true })
    .maybeSingle();

  if (updateError) {
    console.error("Error updating memo:", updateError);
    return NextResponse.json(
      { error: "Failed to update memo" },
      { status: 500 },
    );
  }

  if (!updated) {
    const { data: existing, error: existingError } = await supabase
      .from("memos")
      .select(MEMO_SELECT_FIELDS)
      .eq("id", memoId)
      .eq("user_id", user.id)
      .order("sort_order", { referencedTable: "memo_images", ascending: true })
      .maybeSingle();

    if (existingError || !existing) {
      return NextResponse.json({ error: "Not found" }, { status: 404 });
    }

    const memo = await buildMemoFromRow(supabase, existing as MemoRow);
    const jsonResponse = NextResponse.json(
      { error: "Conflict", memo },
      { status: 409 },
    );
    response.cookies.getAll().forEach((cookie) => {
      jsonResponse.cookies.set(cookie);
    });
    return jsonResponse;
  }

  if (shouldUpdateImages) {
    const { error: deleteError } = await supabase
      .from("memo_images")
      .delete()
      .eq("memo_id", memoId);

    if (deleteError) {
      console.error("Error updating memo images:", deleteError);
      return NextResponse.json(
        { error: "Failed to update memo images" },
        { status: 500 },
      );
    }

    if (normalizedImageUrls.length > 0) {
      const { error: insertError } = await supabase.from("memo_images").insert(
        normalizedImageUrls.map((url, index) => ({
          memo_id: memoId,
          url,
          sort_order: index,
        })),
      );

      if (insertError) {
        console.error("Error updating memo images:", insertError);
        return NextResponse.json(
          { error: "Failed to update memo images" },
          { status: 500 },
        );
      }
    }
  }

  const memoRow = shouldUpdateImages
    ? await supabase
        .from("memos")
        .select(MEMO_SELECT_FIELDS)
        .eq("id", memoId)
        .eq("user_id", user.id)
        .order("sort_order", { referencedTable: "memo_images", ascending: true })
        .maybeSingle()
    : { data: updated, error: null };

  if (memoRow.error || !memoRow.data) {
    return NextResponse.json({ error: "Failed to load memo" }, { status: 500 });
  }

  const memo = await buildMemoFromRow(supabase, memoRow.data as MemoRow);
  const jsonResponse = NextResponse.json({ memo });
  response.cookies.getAll().forEach((cookie) => {
    jsonResponse.cookies.set(cookie);
  });

  return jsonResponse;
}
