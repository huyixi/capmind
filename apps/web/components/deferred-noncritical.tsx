"use client";

import dynamic from "next/dynamic";
import { useEffect, useState } from "react";

type IdleCallbackWindow = Window & {
  requestIdleCallback?: (
    callback: () => void,
    options?: { timeout: number },
  ) => number;
  cancelIdleCallback?: (handle: number) => void;
};

const DeferredPwaUpdate = dynamic(() => import("@/components/pwa-update"), {
  ssr: false,
});

const DeferredAnalytics = dynamic(
  () => import("@vercel/analytics/next").then((mod) => mod.Analytics),
  { ssr: false },
);

export function DeferredNonCritical() {
  const [isIdle, setIsIdle] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const activate = () => {
      if (cancelled) return;
      setIsIdle(true);
    };
    const idleWindow = window as IdleCallbackWindow;
    if (typeof idleWindow.requestIdleCallback === "function") {
      const idleHandle = idleWindow.requestIdleCallback(
        () => {
          activate();
        },
        { timeout: 2000 },
      );
      return () => {
        cancelled = true;
        if (typeof idleWindow.cancelIdleCallback === "function") {
          idleWindow.cancelIdleCallback(idleHandle);
        }
      };
    }

    const timeoutHandle = window.setTimeout(activate, 300);
    return () => {
      cancelled = true;
      window.clearTimeout(timeoutHandle);
    };
  }, []);

  if (!isIdle) return null;

  return (
    <>
      <DeferredPwaUpdate />
      <DeferredAnalytics />
    </>
  );
}
