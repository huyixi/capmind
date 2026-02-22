const VERSION_PATTERN = /^\d+$/;
const DATE_PARTS = [
  "year",
  "month",
  "day",
  "hour",
  "minute",
  "second",
] as const;

type TimestampTimeZone = "local" | "utc";

const createTimestampFormatter = (timeZone?: "UTC") =>
  new Intl.DateTimeFormat("en-CA", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    timeZone,
  });

const localTimestampFormatter = createTimestampFormatter();
const utcTimestampFormatter = createTimestampFormatter("UTC");

const formatTimestampWithFormatter = (
  value: string,
  timeZone: TimestampTimeZone,
): string => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  const formatter =
    timeZone === "utc" ? utcTimestampFormatter : localTimestampFormatter;
  const parts = formatter.formatToParts(date);
  const partValues = DATE_PARTS.reduce<Record<string, string>>((acc, part) => {
    const partValue = parts.find((item) => item.type === part)?.value;
    acc[part] = partValue ?? "";
    return acc;
  }, {});

  return `${partValues.year}-${partValues.month}-${partValues.day} ${partValues.hour}:${partValues.minute}:${partValues.second}`;
};

export const formatTimestampLocal = (value: string): string =>
  formatTimestampWithFormatter(value, "local");

export const formatTimestampUtc = (value: string): string =>
  formatTimestampWithFormatter(value, "utc");

export const normalizeMemoVersion = (
  value: string | number | null | undefined,
): string => {
  if (typeof value === "string") return value;
  if (typeof value === "number" && Number.isFinite(value)) {
    return value.toString();
  }
  return "";
};

export const normalizeExpectedVersion = (
  value: string | number | null | undefined,
): string => {
  const normalized = normalizeMemoVersion(value);
  return VERSION_PATTERN.test(normalized) ? normalized : "0";
};

export const nextMemoVersion = (
  value: string | number | null | undefined,
): string => (BigInt(normalizeExpectedVersion(value)) + BigInt(1)).toString();
