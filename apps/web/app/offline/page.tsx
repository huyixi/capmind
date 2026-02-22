export const metadata = {
  title: "Offline",
};

export default function OfflinePage() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 px-6 text-center">
      <h1 className="text-2xl font-semibold">You are offline</h1>
      <p className="max-w-md text-sm text-muted-foreground">
        This page is available offline. Please check your connection and try again.
      </p>
    </main>
  );
}
