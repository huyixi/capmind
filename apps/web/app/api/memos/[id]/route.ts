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
    response.cookies.getAll().forEach((cookie) => {
      jsonResponse.cookies.set(cookie);
    });
    return jsonResponse;
  }

  if (rpcResult.status !== "updated" || !rpcResult.memo) {
    return NextResponse.json({ error: "Failed to update memo" }, { status: 500 });
  }

  const memo = await buildMemoFromRow(supabase, rpcResult.memo);
  const jsonResponse = NextResponse.json({ memo });
  response.cookies.getAll().forEach((cookie) => {
    jsonResponse.cookies.set(cookie);
  });

  return jsonResponse;
}
