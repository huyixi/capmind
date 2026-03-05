const UPLOAD_ENDPOINT = "/api/memos/images/upload";

function normalizeErrorMessage(status: number, payload: unknown) {
  if (payload && typeof payload === "object" && "error" in payload) {
    const message = (payload as { error?: unknown }).error;
    if (typeof message === "string" && message.trim().length > 0) {
      return message;
    }
  }

  if (status === 401 || status === 403) {
    return "Sign in to submit memos. Your draft is still here.";
  }

  return "Failed to upload images. Please try again.";
}

export async function uploadMemoImages(files: File[]): Promise<string[]> {
  if (files.length === 0) return [];

  const formData = new FormData();
  files.forEach((file) => {
    formData.append("files", file);
  });

  const response = await fetch(UPLOAD_ENDPOINT, {
    method: "POST",
    credentials: "include",
    body: formData,
  });

  const payload = (await response.json().catch(() => null)) as
    | { paths?: unknown; error?: unknown }
    | null;

  if (!response.ok) {
    throw new Error(normalizeErrorMessage(response.status, payload));
  }

  if (!Array.isArray(payload?.paths)) {
    throw new Error("Failed to upload images. Please try again.");
  }

  return payload.paths.filter((path): path is string => typeof path === "string");
}
