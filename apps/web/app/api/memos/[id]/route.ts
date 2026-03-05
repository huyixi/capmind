import { NextRequest, NextResponse } from "next/server";
import { createServerClient } from "@supabase/ssr";
import { buildMemoFromRow, type MemoRow } from "@/lib/memos";
import { MEMO_IMAGES_BUCKET } from "@/lib/memo-constants";
import { extractStoragePath } from "@/lib/supabase/storage";

type MemoUpdateRpcStatus =
  | "updated"
  | "deleted"
  | "restored"
  | "conflict"
  | "not_found";

type MemoUpdateRpcResult = {
  status: MemoUpdateRpcStatus;
  memo_id: string;
  memo: MemoRow | null;
  server_memo: MemoRow | null;
  forked_memo: MemoRow | null;
};

type MemoRestoreRequestBody = {
  action: "restore";
  expectedVersion: string;
  restoredAt?: string;
};

type MemoPatchRequestBody = {
  text?: string;
  expectedVersion?: string;
  imageUrls?: string[];
  action?: "restore";
  restoredAt?: string;
};

const MEMO_SELECT_FIELDS =
  "id, user_id, text, created_at, updated_at, version, deleted_at, memo_images(url, sort_order)";

const copyCookies = (from: NextResponse, to: NextResponse) => {
  from.cookies.getAll().forEach((cookie) => {
    to.cookies.set(cookie);
  });
};

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const resolvedParams = await params;
  const memoId = resolvedParams?.id;
  let body: MemoPatchRequestBody;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON" }, { status: 400 });
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

  if (body.action === "restore") {
    const restoreBody = body as MemoRestoreRequestBody;
    const expectedVersion = restoreBody.expectedVersion;
    const restoredAt = restoreBody.restoredAt ?? new Date().toISOString();
    if (typeof expectedVersion !== "string" || !/^\d+$/.test(expectedVersion)) {
      return NextResponse.json({ error: "Invalid version" }, { status: 400 });
    }

    const nextVersion = String(Number(expectedVersion) + 1);
    const { data: restoredMemo, error: restoreError } = await supabase
      .from("memos")
      .update({
        deleted_at: null,
        updated_at: restoredAt,
        version: nextVersion,
      })
      .eq("id", memoId)
      .eq("user_id", user.id)
      .eq("version", expectedVersion)
      .select(MEMO_SELECT_FIELDS)
      .maybeSingle();

    if (restoreError) {
      console.error("Error restoring memo:", restoreError);
      return NextResponse.json({ error: "Failed to restore memo" }, { status: 500 });
    }

    if (!restoredMemo) {
      return NextResponse.json({ error: "Conflict" }, { status: 409 });
    }

    const memo = await buildMemoFromRow(supabase, restoredMemo as MemoRow);
    const jsonResponse = NextResponse.json({ memo });
    copyCookies(response, jsonResponse);
    return jsonResponse;
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

  const { data: rpcData, error: rpcError } = await supabase.rpc(
    "memo_update_resolve_conflict",
    {
      arg_memo_id: memoId,
      arg_text: trimmedText,
      arg_expected_version: expectedVersion,
      arg_image_urls: shouldUpdateImages ? normalizedImageUrls : null,
    },
  );

  if (rpcError) {
    console.error("Error resolving memo update conflict:", rpcError);
    return NextResponse.json(
      { error: "Failed to update memo" },
      { status: 500 },
    );
  }

  const rpcRows = Array.isArray(rpcData)
    ? (rpcData as MemoUpdateRpcResult[])
    : rpcData
      ? ([rpcData] as MemoUpdateRpcResult[])
      : [];
  const rpcResult = rpcRows[0];
  if (!rpcResult) {
    return NextResponse.json({ error: "Failed to update memo" }, { status: 500 });
  }

  if (rpcResult.status === "not_found") {
    return NextResponse.json({ error: "Not found" }, { status: 404 });
  }

  if (rpcResult.status === "conflict") {
    const serverMemoRow = rpcResult.server_memo;
    const forkedMemoRow = rpcResult.forked_memo;
    const memo = serverMemoRow
      ? await buildMemoFromRow(supabase, serverMemoRow)
      : null;
    const forkedMemo = forkedMemoRow
      ? await buildMemoFromRow(supabase, forkedMemoRow)
      : null;
    const jsonResponse = NextResponse.json(
      { error: "Conflict", memo, forkedMemo },
      { status: 409 },
    );
    copyCookies(response, jsonResponse);
    return jsonResponse;
  }

  if (rpcResult.status !== "updated" || !rpcResult.memo) {
    return NextResponse.json({ error: "Failed to update memo" }, { status: 500 });
  }

  const memo = await buildMemoFromRow(supabase, rpcResult.memo);
  const jsonResponse = NextResponse.json({ memo });
  copyCookies(response, jsonResponse);

  return jsonResponse;
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const resolvedParams = await params;
  const memoId = resolvedParams?.id;

  let body: { expectedVersion?: string; deletedAt?: string };
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON" }, { status: 400 });
  }

  const expectedVersion = body.expectedVersion;
  const deletedAt = body.deletedAt ?? new Date().toISOString();
  if (typeof expectedVersion !== "string" || !/^\d+$/.test(expectedVersion)) {
    return NextResponse.json({ error: "Invalid version" }, { status: 400 });
  }

  const nextVersion = String(Number(expectedVersion) + 1);
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

  const { data: deletedMemo, error: deleteError } = await supabase
    .from("memos")
    .update({
      deleted_at: deletedAt,
      updated_at: deletedAt,
      version: nextVersion,
    })
    .eq("id", memoId)
    .eq("user_id", user.id)
    .eq("version", expectedVersion)
    .select("id")
    .maybeSingle();

  if (deleteError) {
    console.error("Error deleting memo:", deleteError);
    return NextResponse.json({ error: "Failed to delete memo" }, { status: 500 });
  }

  if (!deletedMemo) {
    return NextResponse.json({ error: "Conflict" }, { status: 409 });
  }

  const jsonResponse = NextResponse.json({ ok: true });
  copyCookies(response, jsonResponse);
  return jsonResponse;
}
