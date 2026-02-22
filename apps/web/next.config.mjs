import nextPWA from "next-pwa";

/** @type {import('next').NextConfig} */
const supabaseUrl = process.env.NEXT_PUBLIC_SUPABASE_URL;

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
