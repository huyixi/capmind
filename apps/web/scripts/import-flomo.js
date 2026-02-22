#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const crypto = require("crypto");
const { createClient } = require("@supabase/supabase-js");

const DEFAULT_USER_ID = "1c4ecf49-4bb4-4d82-bff7-284f1b00e4f8";
const DEFAULT_HTML_PATH = path.join(process.cwd(), "flomo-data/index.html");
const DEFAULT_BUCKET = "memo-images";
const DEFAULT_IMAGE_PREFIX = "flomo";

const TIME_PATTERN =
  /^(\d{4})-(\d{2})-(\d{2})\s+(\d{2}):(\d{2})(?::(\d{2}))?$/;

const CONTENT_TYPE_BY_EXT = {
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".png": "image/png",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".bmp": "image/bmp",
  ".svg": "image/svg+xml",
  ".heic": "image/heic",
  ".heif": "image/heif",
};

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (!arg.startsWith("--")) continue;
    const key = arg.slice(2);
    const next = argv[i + 1];
    if (!next || next.startsWith("--")) {
      args[key] = true;
      continue;
    }
    args[key] = next;
    i += 1;
  }
  return args;
}

function loadEnvFile(filePath) {
  if (!fs.existsSync(filePath)) return;
  const content = fs.readFileSync(filePath, "utf8");
  for (const line of content.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const idx = trimmed.indexOf("=");
    if (idx === -1) continue;
    const key = trimmed.slice(0, idx).trim();
    let value = trimmed.slice(idx + 1).trim();
    if (
      (value.startsWith("\"") && value.endsWith("\"")) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) {
      process.env[key] = value;
    }
  }
}

function parseShanghaiToIso(value) {
  if (!value) return null;
  const match = value.trim().match(TIME_PATTERN);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6] ?? "0");
  const utcMs = Date.UTC(year, month - 1, day, hour - 8, minute, second);
  return new Date(utcMs).toISOString();
}

function normalizeText(value) {
  const raw = value.replace(/\u00a0/g, " ");
  const lines = raw.replace(/\r/g, "").split("\n");
  const cleaned = [];
  for (const line of lines) {
    cleaned.push(line.trim());
  }
  while (cleaned.length > 0 && cleaned[0] === "") cleaned.shift();
  while (cleaned.length > 0 && cleaned[cleaned.length - 1] === "")
    cleaned.pop();
  const collapsed = [];
  for (let i = 0; i < cleaned.length; i += 1) {
    const line = cleaned[i];
    if (line !== "") {
      collapsed.push(line);
      continue;
    }
  }
  return collapsed.join("\n");
}

function stripTags(value) {
  return value.replace(/<[^>]+>/g, "");
}

function decodeHtmlEntities(value) {
  return value
    .replace(/&nbsp;/gi, " ")
    .replace(/&amp;/gi, "&")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
    .replace(/&quot;/gi, "\"")
    .replace(/&#39;/gi, "'")
    .replace(/&#(\d+);/g, (_, code) => String.fromCodePoint(Number(code)))
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) =>
      String.fromCodePoint(parseInt(hex, 16)),
    );
}

function htmlToText(html) {
  if (!html) return "";
  let text = html;
  text = text.replace(/<li[^>]*>\s*<p[^>]*>/gi, "<li>");
  text = text.replace(/<\/p>\s*<\/li>/gi, "</li>");
  text = text.replace(/<br\s*\/?>/gi, "\n");
  text = text.replace(/<li[^>]*>/gi, "\n- ");
  text = text.replace(/<\/li>/gi, "\n");
  text = text.replace(/<p[^>]*>/gi, "\n");
  text = text.replace(/<\/p>/gi, "\n");
  text = text.replace(/<div[^>]*>/gi, "\n");
  text = text.replace(/<\/div>/gi, "\n");
  text = text.replace(/<[^>]+>/g, "");
  text = decodeHtmlEntities(text);
  return normalizeText(text);
}

function extractMemoBlocks(html) {
  const blocks = [];
  const startRegex =
    /<div\b[^>]*class=["'][^"']*\bmemo\b[^"']*["'][^>]*>/gi;
  let match = null;
  while ((match = startRegex.exec(html))) {
    let depth = 1;
    const tagRegex = /<\/?div\b[^>]*>/gi;
    tagRegex.lastIndex = startRegex.lastIndex;
    let endIndex = -1;
    while (depth > 0) {
      const tagMatch = tagRegex.exec(html);
      if (!tagMatch) break;
      if (tagMatch[0].startsWith("</div")) {
        depth -= 1;
      } else {
        depth += 1;
      }
      if (depth === 0) {
        endIndex = tagRegex.lastIndex;
        break;
      }
    }
    if (endIndex === -1) break;
    blocks.push(html.slice(match.index, endIndex));
    startRegex.lastIndex = endIndex;
  }
  return blocks;
}

function extractDivInnerHtml(html, className) {
  const startRegex = new RegExp(
    `<div\\b[^>]*class=["'][^"']*\\b${className}\\b[^"']*["'][^>]*>`,
    "i",
  );
  const match = startRegex.exec(html);
  if (!match) return "";
  const startIndex = match.index + match[0].length;
  let depth = 1;
  const tagRegex = /<\/?div\b[^>]*>/gi;
  tagRegex.lastIndex = startIndex;
  while (depth > 0) {
    const tagMatch = tagRegex.exec(html);
    if (!tagMatch) break;
    if (tagMatch[0].startsWith("</div")) {
      depth -= 1;
    } else {
      depth += 1;
    }
    if (depth === 0) {
      return html.slice(startIndex, tagMatch.index);
    }
  }
  return "";
}

function extractImages(html) {
  const images = [];
  const imgRegex = /<img\b[^>]*\bsrc=["']([^"']+)["'][^>]*>/gi;
  let match = null;
  while ((match = imgRegex.exec(html))) {
    images.push(match[1]);
  }
  return images;
}

function extractMemos(html) {
  const blocks = extractMemoBlocks(html);
  const memos = [];
  blocks.forEach((block, index) => {
    const timeHtml = extractDivInnerHtml(block, "time");
    const contentHtml = extractDivInnerHtml(block, "content");
    const filesHtml = extractDivInnerHtml(block, "files");
    const timeText = decodeHtmlEntities(stripTags(timeHtml)).trim();
    const text = htmlToText(contentHtml);
    const images = extractImages(filesHtml).filter(Boolean);
    memos.push({ timeText, text, images, index });
  });
  return memos;
}

function guessContentType(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  return CONTENT_TYPE_BY_EXT[ext] || "application/octet-stream";
}

function uuidFromHash(input) {
  const hash = crypto.createHash("sha1").update(input).digest();
  const bytes = Buffer.from(hash.subarray(0, 16));
  bytes[6] = (bytes[6] & 0x0f) | 0x50;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = bytes.toString("hex");
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20, 32),
  ].join("-");
}

async function mapLimit(items, limit, mapper) {
  const results = new Array(items.length);
  let index = 0;

  async function worker() {
    while (index < items.length) {
      const current = index;
      index += 1;
      results[current] = await mapper(items[current], current);
    }
  }

  const workers = [];
  const count = Math.max(1, Math.min(limit, items.length));
  for (let i = 0; i < count; i += 1) {
    workers.push(worker());
  }
  await Promise.all(workers);
  return results;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log(`Usage:
  node scripts/import-flomo.js [options]

Options:
  --html <path>           HTML export path (default: flomo-data/index.html)
  --files-dir <path>      Directory that contains the file/ folder
  --user-id <uuid>        Supabase user id (default: ${DEFAULT_USER_ID})
  --bucket <name>         Storage bucket (default: ${DEFAULT_BUCKET})
  --image-prefix <path>   Storage prefix inside bucket (default: ${DEFAULT_IMAGE_PREFIX})
  --start <n>             Skip first n memos
  --limit <n>             Only import n memos
  --dry-run               Parse and report, no writes
`);
    return;
  }

  loadEnvFile(path.join(process.cwd(), ".env.local"));
  loadEnvFile(path.join(process.cwd(), ".env"));

  const supabaseUrl =
    process.env.NEXT_PUBLIC_SUPABASE_URL || process.env.SUPABASE_URL;
  const serviceKey = process.env.SUPABASE_SERVICE_ROLE_KEY;
  const anonKey = process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY;
  const supabaseKey = serviceKey || anonKey;
  if (!supabaseUrl || !supabaseKey) {
    console.error(
      "Missing NEXT_PUBLIC_SUPABASE_URL and/or SUPABASE_SERVICE_ROLE_KEY or NEXT_PUBLIC_SUPABASE_ANON_KEY.",
    );
    process.exit(1);
  }
  if (!serviceKey) {
    console.warn(
      "Using NEXT_PUBLIC_SUPABASE_ANON_KEY. Import may fail if RLS blocks inserts.",
    );
  }

  const userId = args["user-id"] || process.env.IMPORT_USER_ID || DEFAULT_USER_ID;
  const htmlPath = args.html || DEFAULT_HTML_PATH;
  const filesDir = args["files-dir"] || path.dirname(htmlPath);
  const bucket = args.bucket || DEFAULT_BUCKET;
  const imagePrefix = args["image-prefix"] || DEFAULT_IMAGE_PREFIX;
  const dryRun = Boolean(args["dry-run"]);
  const start = args.start ? Number(args.start) : 0;
  const limit = args.limit ? Number(args.limit) : null;

  if (!fs.existsSync(htmlPath)) {
    console.error(`HTML not found at ${htmlPath}`);
    process.exit(1);
  }

  const html = fs.readFileSync(htmlPath, "utf8");
  const memos = extractMemos(html);

  const sliced = memos.slice(start, limit ? start + limit : undefined);
  console.log(`Found ${memos.length} memos, importing ${sliced.length}.`);

  if (dryRun) {
    const sample = sliced.slice(0, 3).map((memo) => ({
      timeText: memo.timeText,
      textPreview: memo.text.slice(0, 120),
      imageCount: memo.images.length,
    }));
    console.log("Dry run sample:", sample);
    return;
  }

  const supabase = createClient(supabaseUrl, supabaseKey, {
    auth: { persistSession: false, autoRefreshToken: false },
  });

  const imageCache = new Map();
  const seenMemoIds = new Set();

  const ensureImage = async (src) => {
    if (imageCache.has(src)) return imageCache.get(src);
    if (/^https?:\/\//i.test(src)) {
      imageCache.set(src, null);
      return null;
    }
    const sanitized = src.replace(/^\/+/, "");
    const relative = sanitized.replace(/^file\//, "");
    const localPath = path.join(filesDir, sanitized);
    if (!fs.existsSync(localPath)) {
      console.warn(`Missing image: ${localPath}`);
      imageCache.set(src, null);
      return null;
    }
    const storagePath = `${userId}/${imagePrefix}/${relative}`;
    const buffer = fs.readFileSync(localPath);
    const contentType = guessContentType(localPath);
    const { error } = await supabase.storage.from(bucket).upload(storagePath, buffer, {
      contentType,
      upsert: false,
    });
    if (error && error.statusCode !== 409) {
      console.warn(`Upload failed for ${src}: ${error.message}`);
      imageCache.set(src, null);
      return null;
    }
    imageCache.set(src, storagePath);
    return storagePath;
  };

  let successCount = 0;
  let skippedCount = 0;
  for (let i = 0; i < sliced.length; i += 1) {
    const memo = sliced[i];
    const createdAt = parseShanghaiToIso(memo.timeText) || new Date().toISOString();
    const memoKey = `${userId}|${memo.timeText}|${memo.text}|${memo.images.join(",")}`;
    const memoId = uuidFromHash(memoKey);
    if (seenMemoIds.has(memoId)) {
      skippedCount += 1;
      continue;
    }
    seenMemoIds.add(memoId);
    const text = memo.text ? memo.text.trim() : "";
    const memoRow = {
      id: memoId,
      user_id: userId,
      text,
      created_at: createdAt,
      updated_at: createdAt,
      version: "1",
      deleted_at: null,
    };

    const { error: memoError } = await supabase.from("memos").insert(memoRow);
    if (memoError) {
      if (memoError.code === "23505") {
        skippedCount += 1;
        continue;
      }
      console.warn(`Failed to insert memo ${i + 1}: ${memoError.message}`);
      continue;
    }

    if (memo.images.length > 0) {
      const uploaded = await mapLimit(memo.images, 3, ensureImage);
      const paths = uploaded.filter(Boolean);
      if (paths.length > 0) {
        const imageRows = paths.map((url, idx) => ({
          memo_id: memoId,
          url,
          sort_order: idx,
        }));
        const { error: imageError } = await supabase
          .from("memo_images")
          .insert(imageRows);
        if (imageError) {
          console.warn(`Failed to insert images for memo ${memoId}: ${imageError.message}`);
        }
      }
    }

    successCount += 1;
    if ((i + 1) % 50 === 0 || i === sliced.length - 1) {
      console.log(
        `Imported ${i + 1}/${sliced.length} (success ${successCount}, skipped ${skippedCount}).`,
      );
    }
  }

  console.log(
    `Done. Imported ${successCount}/${sliced.length} memos, skipped ${skippedCount}.`,
  );
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
