import { NextRequest, NextResponse } from "next/server"
import { createServerClient } from "@supabase/ssr"
import { MEMOS_PAGE_SIZE, MEMO_IMAGES_BUCKET } from "@/lib/memo-constants"
import { fetchMemoPage } from "@/lib/memo-queries"
import {
  buildMemoFromRow,
  MEMO_BASE_SELECT_FIELDS,
  MEMO_SELECT_FIELDS,
  type MemoRow,
} from "@/lib/memos"
import { extractStoragePath } from "@/lib/supabase/storage"

export const dynamic = "force-dynamic"

const MAX_PAGE_SIZE = 100

async function createAuthorizedClient(request: NextRequest) {
  const response = NextResponse.next()
  const supabase = createServerClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!,
    {
      cookies: {
        getAll() {
          return request.cookies.getAll()
        },
        setAll(cookiesToSet) {
          cookiesToSet.forEach(({ name, value, options }) =>
            response.cookies.set(name, value, options)
          )
        },
      },
    }
  )

  const {
    data: { user },
    error: userError,
  } = await supabase.auth.getUser()

  return { response, supabase, user, userError }
}

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl
  const pageParam = Number(searchParams.get("page") ?? "0")
  const pageSizeParam = Number(searchParams.get("pageSize") ?? MEMOS_PAGE_SIZE)
  const trashParam = searchParams.get("trash")
  const isTrashView = trashParam === "1" || trashParam === "true"
  const prefetchParam = searchParams.get("prefetch")
  const shouldPrefetch = prefetchParam === "1" || prefetchParam === "true"

  const page = Number.isFinite(pageParam) && pageParam >= 0 ? pageParam : 0
  const pageSize =
    Number.isFinite(pageSizeParam) && pageSizeParam > 0
      ? Math.min(pageSizeParam, MAX_PAGE_SIZE)
      : MEMOS_PAGE_SIZE

  const authStart = Date.now()
  const { response, supabase, user, userError } =
    await createAuthorizedClient(request)
  const authMs = Date.now() - authStart

  if (userError || !user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 })
  }

  const { memos, error, timings } = await fetchMemoPage({
    supabase,
    userId: user.id,
    page,
    pageSize,
    isTrashView,
  })

  if (error) {
    console.error("Error fetching memos:", error)
    return NextResponse.json({ error: "Failed to fetch memos" }, { status: 500 })
  }

  let prefetched: typeof memos = []
  let prefetchMs: number | null = null
  if (shouldPrefetch && memos.length === pageSize) {
    const result = await fetchMemoPage({
      supabase,
      userId: user.id,
      page: page + 1,
      pageSize,
      isTrashView,
    })
    if (result.error) {
      console.error("Error prefetching memos:", result.error)
    } else {
      prefetched = result.memos
      prefetchMs = result.timings.totalMs
    }
  }

  const jsonResponse = NextResponse.json(
    shouldPrefetch ? { memos, prefetched } : { memos },
  )
  const timingParts = [
    `auth;dur=${authMs.toFixed(2)}`,
    `memos;dur=${timings.memosMs.toFixed(2)}`,
    `memo_images;dur=${timings.memoImagesMs.toFixed(2)}`,
    `build;dur=${timings.buildMs.toFixed(2)}`,
    `total;dur=${timings.totalMs.toFixed(2)}`,
  ]
  if (prefetchMs !== null) {
    timingParts.push(`prefetch;dur=${prefetchMs.toFixed(2)}`)
  }
  jsonResponse.headers.set("Server-Timing", timingParts.join(", "))
  response.cookies.getAll().forEach((cookie) => {
    jsonResponse.cookies.set(cookie)
  })

  return jsonResponse
}

export async function POST(request: NextRequest) {
  let body: { text?: string; imageUrls?: string[] }
  try {
    body = await request.json()
  } catch {
    return NextResponse.json({ error: "Invalid JSON" }, { status: 400 })
  }

  if (typeof body.text !== "string") {
    return NextResponse.json({ error: "Invalid payload" }, { status: 400 })
  }
  if (
    body.imageUrls !== undefined &&
    (!Array.isArray(body.imageUrls) ||
      !body.imageUrls.every((item) => typeof item === "string"))
  ) {
    return NextResponse.json({ error: "Invalid payload" }, { status: 400 })
  }

  const trimmedText = body.text.trim()
  const imageUrls = (body.imageUrls ?? [])
    .map((raw) => extractStoragePath(raw, MEMO_IMAGES_BUCKET) ?? raw.trim())
    .filter((value) => value.length > 0)

  if (trimmedText.length === 0 && imageUrls.length === 0) {
    return NextResponse.json({ error: "Memo must have content" }, { status: 400 })
  }

  const { response, supabase, user, userError } =
    await createAuthorizedClient(request)

  if (userError || !user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 })
  }

  const { data: insertedMemo, error: insertError } = await supabase
    .from("memos")
    .insert({
      user_id: user.id,
      text: trimmedText,
    })
    .select(MEMO_BASE_SELECT_FIELDS)
    .single()

  if (insertError || !insertedMemo) {
    console.error("Error creating memo:", insertError)
    return NextResponse.json({ error: "Failed to create memo" }, { status: 500 })
  }

  if (imageUrls.length > 0) {
    const { error: imageError } = await supabase.from("memo_images").insert(
      imageUrls.map((url, index) => ({
        memo_id: insertedMemo.id,
        url,
        sort_order: index,
      }))
    )

    if (imageError) {
      console.error("Error saving memo images:", imageError)
    }
  }

  const { data: createdMemo, error: fetchError } = await supabase
    .from("memos")
    .select(MEMO_SELECT_FIELDS)
    .eq("id", insertedMemo.id)
    .eq("user_id", user.id)
    .order("sort_order", {
      referencedTable: "memo_images",
      ascending: true,
    })
    .maybeSingle()

  if (fetchError) {
    console.error("Error fetching created memo:", fetchError)
  }

  const memo = await buildMemoFromRow(
    supabase,
    (createdMemo ?? insertedMemo) as MemoRow
  )

  const jsonResponse = NextResponse.json({ memo })
  response.cookies.getAll().forEach((cookie) => {
    jsonResponse.cookies.set(cookie)
  })
  return jsonResponse
}
