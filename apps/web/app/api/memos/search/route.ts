import { NextRequest, NextResponse } from "next/server";
import { createServerClient } from "@supabase/ssr";
import {
  buildMemosFromRows,
  MEMO_BASE_WITH_IMAGE_COUNT_SELECT_FIELDS,
  type MemoRow,
} from "@/lib/memos";

export const dynamic = "force-dynamic";

const DEFAULT_SEARCH_LIMIT = 50;
const MAX_SEARCH_LIMIT = 200;

const escapeLikePattern = (value: string) => value.replace(/([%_\\])/g, "\\$1");

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl;
  const query = searchParams.get("q")?.trim() ?? "";
  const limitParam = Number(searchParams.get("limit") ?? DEFAULT_SEARCH_LIMIT);

  const limit =
    Number.isFinite(limitParam) && limitParam > 0
      ? Math.min(limitParam, MAX_SEARCH_LIMIT)
      : DEFAULT_SEARCH_LIMIT;

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

  if (!query) {
    return NextResponse.json({ memos: [] });
  }

  const pattern = `%${escapeLikePattern(query)}%`;

  const { data, error } = await supabase
    .from("memos")
    .select(MEMO_BASE_WITH_IMAGE_COUNT_SELECT_FIELDS)
    .eq("user_id", user.id)
    .is("deleted_at", null)
    .ilike("text", pattern)
    .order("created_at", { ascending: false })
    .limit(limit);

  if (error) {
    console.error("Error searching memos:", error);
    return NextResponse.json(
      { error: "Failed to search memos" },
      { status: 500 },
    );
  }

  const memoRows = (data ?? []) as MemoRow[];
  const memos = await buildMemosFromRows(supabase, memoRows, {
    resolveImages: false,
  });

  const jsonResponse = NextResponse.json({ memos });
  response.cookies.getAll().forEach((cookie) => {
    jsonResponse.cookies.set(cookie);
  });

  return jsonResponse;
}
