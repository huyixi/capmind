const optimisticImages = new Map<string, string[]>();

export const registerOptimisticImages = (
  clientId: string,
  urls: string[],
): void => {
  if (!clientId || urls.length === 0) return;
  optimisticImages.set(clientId, urls);
};

export const cleanupOptimisticImages = (clientId: string): void => {
  const urls = optimisticImages.get(clientId);
  if (!urls) return;
  urls.forEach((url) => URL.revokeObjectURL(url));
  optimisticImages.delete(clientId);
};

export const cleanupAllOptimisticImages = (): void => {
  optimisticImages.forEach((urls) => {
    urls.forEach((url) => URL.revokeObjectURL(url));
  });
  optimisticImages.clear();
};
