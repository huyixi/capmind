import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const webRoot = path.resolve(__dirname, "..");
const monorepoRoot = path.resolve(webRoot, "../..");

function applyEnvFile(targetEnv, filePath) {
  if (!fs.existsSync(filePath)) {
    return;
  }

  const content = fs.readFileSync(filePath, "utf8");
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }

    const normalized = line.startsWith("export ")
      ? line.slice("export ".length)
      : line;
    const eqIndex = normalized.indexOf("=");
    if (eqIndex <= 0) {
      continue;
    }

    const key = normalized.slice(0, eqIndex).trim();
    let value = normalized.slice(eqIndex + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    if (!targetEnv[key]) {
      targetEnv[key] = value;
    }
  }
}

function applySupabaseAliases(targetEnv) {
  if (!targetEnv.SUPABASE_URL && targetEnv.NEXT_PUBLIC_SUPABASE_URL) {
    targetEnv.SUPABASE_URL = targetEnv.NEXT_PUBLIC_SUPABASE_URL;
  }
  if (!targetEnv.NEXT_PUBLIC_SUPABASE_URL && targetEnv.SUPABASE_URL) {
    targetEnv.NEXT_PUBLIC_SUPABASE_URL = targetEnv.SUPABASE_URL;
  }

  if (!targetEnv.SUPABASE_ANON_KEY && targetEnv.NEXT_PUBLIC_SUPABASE_ANON_KEY) {
    targetEnv.SUPABASE_ANON_KEY = targetEnv.NEXT_PUBLIC_SUPABASE_ANON_KEY;
  }
  if (!targetEnv.NEXT_PUBLIC_SUPABASE_ANON_KEY && targetEnv.SUPABASE_ANON_KEY) {
    targetEnv.NEXT_PUBLIC_SUPABASE_ANON_KEY = targetEnv.SUPABASE_ANON_KEY;
  }
}

const env = { ...process.env };
applyEnvFile(env, path.join(monorepoRoot, ".env.local"));
applyEnvFile(env, path.join(monorepoRoot, ".env"));
applyEnvFile(env, path.join(webRoot, ".env.local"));
applyEnvFile(env, path.join(webRoot, ".env"));
applySupabaseAliases(env);

const [command, ...args] = process.argv.slice(2);
if (!command) {
  console.error("Usage: node scripts/with-root-env.mjs <command> [...args]");
  process.exit(1);
}

const child = spawn(command, args, {
  stdio: "inherit",
  env,
  shell: false,
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
