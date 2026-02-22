import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import nextPWA from "next-pwa";

/** @type {import('next').NextConfig} */
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const webRoot = __dirname;
const monorepoRoot = path.resolve(__dirname, "../..");

const applyEnvFile = (filePath) => {
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

    if (!process.env[key]) {
      process.env[key] = value;
    }
  }
};

const applySupabaseAliases = () => {
  if (!process.env.SUPABASE_URL && process.env.NEXT_PUBLIC_SUPABASE_URL) {
    process.env.SUPABASE_URL = process.env.NEXT_PUBLIC_SUPABASE_URL;
  }
  if (!process.env.NEXT_PUBLIC_SUPABASE_URL && process.env.SUPABASE_URL) {
    process.env.NEXT_PUBLIC_SUPABASE_URL = process.env.SUPABASE_URL;
  }

  if (
    !process.env.SUPABASE_ANON_KEY &&
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY
  ) {
    process.env.SUPABASE_ANON_KEY = process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY;
  }
  if (
    !process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY &&
    process.env.SUPABASE_ANON_KEY
  ) {
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY = process.env.SUPABASE_ANON_KEY;
  }
};

applyEnvFile(path.join(monorepoRoot, ".env.local"));
applyEnvFile(path.join(monorepoRoot, ".env"));
applyEnvFile(path.join(webRoot, ".env.local"));
applyEnvFile(path.join(webRoot, ".env"));
applySupabaseAliases();

const supabaseUrl = process.env.SUPABASE_URL;

let supabasePattern = null;
if (supabaseUrl) {
  try {
    const { protocol, hostname, port } = new URL(supabaseUrl);
    supabasePattern = {
      protocol: protocol.replace(":", ""),
      hostname,
      port: port || undefined,
      pathname: "/storage/v1/object/**",
    };
  } catch {
    // Fall back to wildcard patterns below.
  }
}

const nextConfig = {
  turbopack: {},
  images: {
    remotePatterns: [
      ...(supabasePattern ? [supabasePattern] : []),
      {
        protocol: "https",
        hostname: "supabase.co",
        pathname: "/storage/v1/object/**",
      },
      {
        protocol: "https",
        hostname: "**.supabase.co",
        pathname: "/storage/v1/object/**",
      },
      {
        protocol: "https",
        hostname: "**.supabase.in",
        pathname: "/storage/v1/object/**",
      },
    ],
  },
};

const withPWA = nextPWA({
  dest: "public",
  register: false,
  skipWaiting: false,
  fallbacks: {
    document: "/offline",
  },
  disable: process.env.NODE_ENV === "development",
});

export default withPWA(nextConfig);
