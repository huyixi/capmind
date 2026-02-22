import React from "react";
import type { Metadata } from "next";
import { DeferredNonCritical } from "@/components/deferred-noncritical";
import "./globals.css";

export const metadata: Metadata = {
  title: "Memos",
  description: "huyixi's memos",
  manifest: "/manifest.json",
  icons: {
    icon: [
      {
        url: "/web-app-manifest-192x192.png",
        sizes: "192x192",
        type: "image/png",
      },
      {
        url: "/web-app-manifest-512x512.png",
        sizes: "512x512",
        type: "image/png",
      },
    ],
    apple: "/web-app-manifest-192x192.png",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <head>
        <meta name="apple-mobile-web-app-title" content="huyixi's Memos" />
      </head>
      <body className={`font-sans antialiased`}>
        {children}
        <DeferredNonCritical />
      </body>
    </html>
  );
}
