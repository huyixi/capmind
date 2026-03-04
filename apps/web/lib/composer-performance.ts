"use client";

export type ComposerPerfMetricName =
  | "composer_open_to_focus_ms"
  | "first_keystroke_ready_ms";

export type ComposerPerfMetricDetail = {
  name: ComposerPerfMetricName;
  valueMs: number;
  mode: "create" | "edit";
  recordedAt: string;
};

export const COMPOSER_PERF_EVENT_NAME = "capmind:composer-perf";

export function reportComposerPerfMetric(
  name: ComposerPerfMetricName,
  valueMs: number,
  mode: "create" | "edit",
) {
  if (!Number.isFinite(valueMs) || valueMs < 0) return;

  const detail: ComposerPerfMetricDetail = {
    name,
    valueMs: Number(valueMs.toFixed(2)),
    mode,
    recordedAt: new Date().toISOString(),
  };

  window.dispatchEvent(
    new CustomEvent<ComposerPerfMetricDetail>(COMPOSER_PERF_EVENT_NAME, {
      detail,
    }),
  );

  if (process.env.NODE_ENV !== "production") {
    // Keep local perf debugging visible without adding third-party dependencies.
    console.info(
      `[perf] ${detail.name}=${detail.valueMs}ms mode=${detail.mode}`,
    );
  }
}
