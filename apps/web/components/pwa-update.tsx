"use client";

import { useEffect, useRef, useState } from "react";
import type { Workbox } from "workbox-window";

import { Button } from "@/components/ui/button";

export default function PwaUpdate() {
  const [updateReady, setUpdateReady] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const wbRef = useRef<Workbox | null>(null);

  useEffect(() => {
    if (process.env.NODE_ENV !== "production") {
      return;
    }

    if (!("serviceWorker" in navigator)) {
      return;
    }

    let cancelled = false;
    let wb: Workbox | null = null;

    const onWaiting = () => {
      if (cancelled) {
        return;
      }
      setDismissed(false);
      setUpdateReady(true);
    };
    const onControlling = () => {
      if (!cancelled) {
        window.location.reload();
      }
    };

    const register = async () => {
      try {
        const response = await fetch("/sw.js", {
          method: "HEAD",
          cache: "no-cache",
        });
        if (!response.ok || cancelled) {
          return;
        }

        const { Workbox: WorkboxClass } = await import("workbox-window");
        if (cancelled) {
          return;
        }

        wb = new WorkboxClass("/sw.js");
        wbRef.current = wb;
        wb.addEventListener("waiting", onWaiting);
        wb.addEventListener("controlling", onControlling);

        const registration = await wb.register();
        if (registration?.waiting && !cancelled) {
          setDismissed(false);
          setUpdateReady(true);
        }
      } catch {
        // Ignore missing/invalid service worker.
      }
    };

    void register();

    return () => {
      cancelled = true;
      if (wb) {
        wb.removeEventListener("waiting", onWaiting);
        wb.removeEventListener("controlling", onControlling);
      }
    };
  }, []);

  if (!updateReady || dismissed) {
    return null;
  }

  const refresh = async () => {
    const wb = wbRef.current;
    if (!wb) {
      return;
    }
    await wb.messageSkipWaiting();
  };

  return (
    <div className="fixed inset-x-4 bottom-4 z-50">
      <div className="mx-auto flex max-w-xl items-center justify-between gap-3 rounded-lg border bg-background/95 px-4 py-3 shadow-lg backdrop-blur">
        <div className="text-sm">
          <div className="font-medium">Update available</div>
          <div className="text-muted-foreground">
            Refresh to get the latest version.
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="secondary" onClick={() => setDismissed(true)}>
            Later
          </Button>
          <Button size="sm" onClick={refresh}>
            Update
          </Button>
        </div>
      </div>
    </div>
  );
}
