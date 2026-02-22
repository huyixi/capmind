#!/usr/bin/env node

import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import lighthouse from "lighthouse";
import { launch as launchChrome } from "chrome-launcher";

const DEFAULT_PORT = 3301;
const HEALTH_TIMEOUT_MS = 90_000;
const POLL_INTERVAL_MS = 800;
const REPORT_DIR = path.join(process.cwd(), "artifacts", "perf");
const THRESHOLDS_PATH = path.join(process.cwd(), "scripts", "perf-thresholds.json");
const MEMO_CONTAINER_MANIFEST_KEY = "/components/memo-container.tsx";

function parseArgs(argv) {
  const options = {
    assert: true,
    skipBuild: false,
    port: DEFAULT_PORT,
    path: "/",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--no-assert") {
      options.assert = false;
      continue;
    }
    if (arg === "--skip-build") {
      options.skipBuild = true;
      continue;
    }
    if (arg === "--port") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error("Missing value for --port");
      }
      options.port = Number.parseInt(value, 10);
      index += 1;
      continue;
    }
    if (arg === "--path") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error("Missing value for --path");
      }
      options.path = value.startsWith("/") ? value : `/${value}`;
      index += 1;
      continue;
    }
    throw new Error(`Unknown argument: ${arg}`);
  }

  if (!Number.isFinite(options.port) || options.port <= 0) {
    throw new Error(`Invalid port: ${options.port}`);
  }

  return options;
}

function nowStamp() {
  return new Date().toISOString().replace(/[:.]/g, "-");
}

function metricFromAudit(audits, id, precision = 0) {
  const numericValue = audits[id]?.numericValue;
  if (typeof numericValue !== "number" || Number.isNaN(numericValue)) {
    return null;
  }
  return Number(numericValue.toFixed(precision));
}

function toMilliseconds(value) {
  if (value === null) return null;
  return Math.round(value);
}

async function runCommand(command, args) {
  const child = spawn(command, args, {
    stdio: "inherit",
    shell: false,
  });

  const [code] = await once(child, "exit");
  if (code !== 0) {
    throw new Error(`${command} ${args.join(" ")} exited with code ${code}`);
  }
}

async function waitForServer(url, timeoutMs, serverProcess) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    if (serverProcess.exitCode !== null) {
      throw new Error(
        `Server exited before becoming ready (exit code ${serverProcess.exitCode}).`,
      );
    }

    try {
      const response = await fetch(url, {
        redirect: "manual",
        signal: AbortSignal.timeout(1_000),
      });
      if (response.status >= 200 && response.status < 500) {
        return;
      }
    } catch {
      // Ignore connection errors while server boots.
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }

  throw new Error(`Server did not become ready within ${timeoutMs}ms: ${url}`);
}

async function stopServer(child) {
  if (!child || child.killed || child.exitCode !== null) {
    return;
  }

  child.kill("SIGTERM");
  await Promise.race([
    once(child, "exit"),
    new Promise((resolve) =>
      setTimeout(() => {
        if (!child.killed && child.exitCode === null) {
          child.kill("SIGKILL");
        }
        resolve();
      }, 5_000),
    ),
  ]);
}

async function readThresholds() {
  const content = await readFile(THRESHOLDS_PATH, "utf8");
  return JSON.parse(content);
}

async function resolveMemoContainerEntryBytes() {
  const manifestPath = path.join(
    process.cwd(),
    ".next",
    "server",
    "app",
    "page_client-reference-manifest.js",
  );
  const manifestSource = await readFile(manifestPath, "utf8");
  const keyIndex = manifestSource.indexOf(MEMO_CONTAINER_MANIFEST_KEY);
  if (keyIndex === -1) {
    return null;
  }

  const tail = manifestSource.slice(keyIndex, keyIndex + 1_400);
  const chunkMatch = tail.match(/"chunks":\[(.*?)\]/);
  if (!chunkMatch) {
    return null;
  }

  let parsed;
  try {
    parsed = JSON.parse(`[${chunkMatch[1]}]`);
  } catch {
    return null;
  }

  const chunkPaths = parsed.filter(
    (value) => typeof value === "string" && value.startsWith("static/chunks/"),
  );

  let totalBytes = 0;
  for (const relativePath of chunkPaths) {
    const absolutePath = path.join(process.cwd(), ".next", relativePath);
    const stats = await import("node:fs/promises").then(({ stat }) =>
      stat(absolutePath),
    );
    totalBytes += stats.size;
  }

  return totalBytes;
}

function buildChecks(metrics, thresholds) {
  const checks = [
    {
      label: "Performance score",
      metricKey: "performanceScore",
      thresholdKey: "performanceScoreMin",
      type: "min",
      unit: "",
    },
    {
      label: "FCP",
      metricKey: "firstContentfulPaintMs",
      thresholdKey: "firstContentfulPaintMsMax",
      type: "max",
      unit: "ms",
    },
    {
      label: "LCP",
      metricKey: "largestContentfulPaintMs",
      thresholdKey: "largestContentfulPaintMsMax",
      type: "max",
      unit: "ms",
    },
    {
      label: "INP",
      metricKey: "interactionToNextPaintMs",
      thresholdKey: "interactionToNextPaintMsMax",
      type: "max",
      unit: "ms",
    },
    {
      label: "TBT",
      metricKey: "totalBlockingTimeMs",
      thresholdKey: "totalBlockingTimeMsMax",
      type: "max",
      unit: "ms",
    },
    {
      label: "Speed Index",
      metricKey: "speedIndexMs",
      thresholdKey: "speedIndexMsMax",
      type: "max",
      unit: "ms",
    },
    {
      label: "CLS",
      metricKey: "cumulativeLayoutShift",
      thresholdKey: "cumulativeLayoutShiftMax",
      type: "max",
      unit: "",
    },
    {
      label: "Total Byte Weight",
      metricKey: "totalByteWeightBytes",
      thresholdKey: "totalByteWeightBytesMax",
      type: "max",
      unit: "bytes",
    },
    {
      label: "Memo Container Entry JS",
      metricKey: "memoContainerEntryBytes",
      thresholdKey: "memoContainerEntryBytesMax",
      type: "max",
      unit: "bytes",
    },
  ];

  return checks.map((check) => {
    const value = metrics[check.metricKey];
    const threshold = thresholds[check.thresholdKey];
    const hasValue = typeof value === "number";
    const hasThreshold = typeof threshold === "number";
    const pass =
      !hasValue ||
      !hasThreshold ||
      (check.type === "min" ? value >= threshold : value <= threshold);

    return {
      ...check,
      value,
      threshold,
      pass,
      evaluated: hasValue && hasThreshold,
    };
  });
}

function formatValue(value, unit) {
  if (typeof value !== "number") {
    return "n/a";
  }
  if (!unit) {
    return String(value);
  }
  return `${value} ${unit}`;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const baseUrl = `http://127.0.0.1:${options.port}`;
  const targetUrl = `${baseUrl}${options.path}`;
  const thresholds = await readThresholds();

  if (!options.skipBuild) {
    console.log("Building production bundle...");
    await runCommand("pnpm", ["build"]);
  }

  console.log(`Starting app on ${baseUrl} ...`);
  const server = spawn(
    "pnpm",
    ["start", "-p", String(options.port), "-H", "127.0.0.1"],
    {
    stdio: ["ignore", "pipe", "pipe"],
    shell: false,
    },
  );

  let serverLogs = "";
  server.stdout.on("data", (chunk) => {
    serverLogs += String(chunk);
  });
  server.stderr.on("data", (chunk) => {
    serverLogs += String(chunk);
  });

  let chrome;
  let runError = null;
  try {
    await waitForServer(targetUrl, HEALTH_TIMEOUT_MS, server);
    console.log(`Running Lighthouse for ${targetUrl} ...`);

    chrome = await launchChrome({
      chromeFlags: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage"],
      logLevel: "silent",
    });

    const runnerResult = await lighthouse(targetUrl, {
      port: chrome.port,
      output: "json",
      logLevel: "error",
      onlyCategories: ["performance"],
      formFactor: "mobile",
      screenEmulation: {
        mobile: true,
        width: 390,
        height: 844,
        deviceScaleFactor: 2,
        disabled: false,
      },
    });

    if (!runnerResult?.lhr) {
      throw new Error("Lighthouse did not return an LHR payload.");
    }

    const lhr = runnerResult.lhr;
    const audits = lhr.audits;
    const metrics = {
      performanceScore: Math.round((lhr.categories.performance?.score ?? 0) * 100),
      firstContentfulPaintMs: toMilliseconds(
        metricFromAudit(audits, "first-contentful-paint"),
      ),
      largestContentfulPaintMs: toMilliseconds(
        metricFromAudit(audits, "largest-contentful-paint"),
      ),
      interactionToNextPaintMs: toMilliseconds(
        metricFromAudit(audits, "interaction-to-next-paint") ??
          metricFromAudit(audits, "experimental-interaction-to-next-paint"),
      ),
      totalBlockingTimeMs: toMilliseconds(
        metricFromAudit(audits, "total-blocking-time"),
      ),
      speedIndexMs: toMilliseconds(metricFromAudit(audits, "speed-index")),
      cumulativeLayoutShift: metricFromAudit(
        audits,
        "cumulative-layout-shift",
        3,
      ),
      totalByteWeightBytes: toMilliseconds(
        metricFromAudit(audits, "total-byte-weight"),
      ),
      memoContainerEntryBytes: await resolveMemoContainerEntryBytes(),
    };

    const checks = buildChecks(metrics, thresholds);
    const failed = checks.filter((check) => check.evaluated && !check.pass);
    const stamp = nowStamp();

    await mkdir(REPORT_DIR, { recursive: true });
    const lhrPath = path.join(REPORT_DIR, `lighthouse-${stamp}.json`);
    const summaryPath = path.join(REPORT_DIR, `summary-${stamp}.json`);
    await writeFile(lhrPath, JSON.stringify(lhr, null, 2), "utf8");
    await writeFile(
      summaryPath,
      JSON.stringify(
        {
          targetUrl,
          generatedAt: new Date().toISOString(),
          metrics,
          checks,
          thresholds,
        },
        null,
        2,
      ),
      "utf8",
    );

    console.log("\nPerformance Regression Report");
    for (const check of checks) {
      const comparator = check.type === "min" ? ">=" : "<=";
      const status = check.evaluated ? (check.pass ? "PASS" : "FAIL") : "SKIP";
      console.log(
        `- ${status} ${check.label}: ${formatValue(check.value, check.unit)} ${comparator} ${formatValue(check.threshold, check.unit)}`,
      );
    }
    console.log(`\nSaved full report: ${path.relative(process.cwd(), lhrPath)}`);
    console.log(`Saved summary: ${path.relative(process.cwd(), summaryPath)}`);

    if (options.assert && failed.length > 0) {
      process.exitCode = 1;
      console.error(`\nPerformance regression check failed (${failed.length} checks).`);
    }
  } catch (error) {
    runError = error;
    throw error;
  } finally {
    if (chrome) {
      await chrome.kill();
    }
    await stopServer(server);
    if ((runError || process.exitCode) && serverLogs.trim()) {
      console.error("\nCaptured server logs:");
      console.error(serverLogs.slice(-4_000));
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
