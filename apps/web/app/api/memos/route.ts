import { NextRequest, NextResponse } from "next/server"
import { createServerClient } from "@supabase/ssr"
import { MEMOS_PAGE_SIZE } from "@/lib/memo-constants"
import { fetchMemoPage } from "@/lib/memo-queries"

export const dynamic = "force-dynamic"

const MAX_PAGE_SIZE = 100

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
  const authStart = Date.now()
  const {
    data: { user },
    error: userError,
  } = await supabase.auth.getUser()
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
