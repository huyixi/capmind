#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { createClient } = require("@supabase/supabase-js");

const DEFAULT_USER_ID = "1c4ecf49-4bb4-4d82-bff7-284f1b00e4f8";
const PAGE_SIZE = 500;

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

function normalizeVersion(value) {
  if (typeof value === "string" && /^\d+$/.test(value)) {
    return BigInt(value);
  }
  if (typeof value === "number" && Number.isFinite(value)) {
    return BigInt(Math.floor(value));
  }
  return 0n;
}

function removeEmptyLines(text) {
  if (!text) return text;
  const lines = text.replace(/\r/g, "").split("\n");
  const cleaned = lines.filter((line) => line.trim() !== "");
  return cleaned.join("\n");
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log(`Usage:
  node scripts/cleanup-memo-newlines.js [options]

Options:
  --user-id <uuid>        Supabase user id (default: ${DEFAULT_USER_ID})
  --dry-run               Show how many would change without updating
  --limit <n>             Only process first n memos
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
      "Using NEXT_PUBLIC_SUPABASE_ANON_KEY. Updates may fail if RLS blocks updates.",
    );
  }

  const userId = args["user-id"] || process.env.IMPORT_USER_ID || DEFAULT_USER_ID;
  const dryRun = Boolean(args["dry-run"]);
  const limit = args.limit ? Number(args.limit) : null;

  const supabase = createClient(supabaseUrl, supabaseKey, {
    auth: { persistSession: false, autoRefreshToken: false },
  });

  let offset = 0;
  let total = 0;
  let changed = 0;
  let updated = 0;

  while (true) {
    const { data, error } = await supabase
      .from("memos")
      .select("id, text, version")
      .eq("user_id", userId)
      .order("created_at", { ascending: false })
      .range(offset, offset + PAGE_SIZE - 1);

    if (error) {
      console.error("Failed to fetch memos:", error.message);
      process.exit(1);
    }

    const rows = data ?? [];
    if (rows.length === 0) break;

    for (const row of rows) {
      total += 1;
      if (limit && total > limit) {
        console.log(`Reached limit ${limit}.`);
        console.log(`Scanned ${total - 1}, changed ${changed}, updated ${updated}.`);
        return;
      }
      const nextText = removeEmptyLines(row.text ?? "");
      if (nextText === row.text) {
        continue;
      }
      changed += 1;
      if (dryRun) continue;

      const nextVersion = (normalizeVersion(row.version) + 1n).toString();
      const { error: updateError } = await supabase
        .from("memos")
        .update({
          text: nextText,
          updated_at: new Date().toISOString(),
          version: nextVersion,
        })
        .eq("id", row.id)
        .eq("user_id", userId);

      if (updateError) {
        console.warn(`Failed to update memo ${row.id}: ${updateError.message}`);
      } else {
        updated += 1;
      }
    }

    offset += PAGE_SIZE;
  }

  if (dryRun) {
    console.log(`Scanned ${total} memos, ${changed} would change.`);
  } else {
    console.log(`Scanned ${total} memos, updated ${updated}/${changed}.`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
