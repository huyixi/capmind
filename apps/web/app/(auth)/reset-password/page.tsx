"use client";

import React, { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { FileText, Loader2 } from "lucide-react";
import { type AuthChangeEvent, type Session } from "@supabase/supabase-js";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { createClient } from "@/lib/supabase/client";

type PageMode = "request" | "email-sent" | "update" | "success";

const MIN_PASSWORD_LENGTH = 6;

function hasRecoveryIndicator() {
  if (typeof window === "undefined") {
    return false;
  }

  const hash = window.location.hash;
  const search = window.location.search;
  return (
    hash.includes("type=recovery") ||
    search.includes("type=recovery") ||
    search.includes("code=")
  );
}

export default function ResetPasswordPage() {
  const router = useRouter();
  const supabase = useMemo(() => createClient(), []);
  const [mode, setMode] = useState<PageMode>("request");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [isCheckingRecovery, setIsCheckingRecovery] = useState(true);

  useEffect(() => {
    let isCancelled = false;

    const markUpdateMode = () => {
      if (!isCancelled) {
        setMode("update");
        setError(null);
      }
    };

    const resolveRecoveryState = async () => {
      const recoveryInUrl = hasRecoveryIndicator();
      const url = new URL(window.location.href);
      const code = url.searchParams.get("code");

      try {
        if (code) {
          const { error: exchangeError } =
            await supabase.auth.exchangeCodeForSession(code);

          if (exchangeError) {
            throw exchangeError;
          }
        }

        const {
          data: { session },
        } = await supabase.auth.getSession();

        if (isCancelled) {
          return;
        }

        if (recoveryInUrl && session) {
          markUpdateMode();
          return;
        }

        if (recoveryInUrl && !session) {
          setError(
            "This reset link is invalid or has expired. Request a new password reset email.",
          );
        }
      } catch {
        if (!isCancelled && recoveryInUrl) {
          setError(
            "This reset link is invalid or has expired. Request a new password reset email.",
          );
        }
      } finally {
        if (!isCancelled) {
          setIsCheckingRecovery(false);
        }
      }
    };

    resolveRecoveryState();

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange(
      (event: AuthChangeEvent, session: Session | null) => {
        if (isCancelled) {
          return;
        }

        if (event === "PASSWORD_RECOVERY" || (hasRecoveryIndicator() && session)) {
          markUpdateMode();
          setIsCheckingRecovery(false);
        }
      },
    );

    return () => {
      isCancelled = true;
      subscription.unsubscribe();
    };
  }, [supabase]);

  const handleRequestReset = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    try {
      const redirectTo = `${window.location.origin}/reset-password`;
      const { error } = await supabase.auth.resetPasswordForEmail(email, {
        redirectTo,
      });

      if (error) {
        setError(error.message);
        return;
      }

      setMode("email-sent");
    } catch (err) {
      if (err instanceof TypeError && err.message.includes("Failed to fetch")) {
        setError(
          "Cannot reach Supabase. Check SUPABASE_URL/SUPABASE_ANON_KEY in capmind/.env.local and restart dev server.",
        );
        return;
      }

      setError(
        err instanceof Error
          ? err.message
          : "Unexpected password reset request error",
      );
    } finally {
      setLoading(false);
    }
  };

  const handleUpdatePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    if (password !== confirmPassword) {
      setError("Passwords do not match");
      setLoading(false);
      return;
    }

    if (password.length < MIN_PASSWORD_LENGTH) {
      setError(`Password must be at least ${MIN_PASSWORD_LENGTH} characters`);
      setLoading(false);
      return;
    }

    try {
      const { error } = await supabase.auth.updateUser({ password });

      if (error) {
        setError(error.message);
        return;
      }

      await supabase.auth.signOut();
      setMode("success");
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Unexpected password update error",
      );
    } finally {
      setLoading(false);
    }
  };

  const renderRequestForm = () => (
    <>
      <div className="flex flex-col items-center space-y-2 text-center">
        <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-primary">
          <FileText className="h-6 w-6 text-primary-foreground" />
        </div>
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">
          Reset your password
        </h1>
        <p className="text-sm text-muted-foreground">
          Enter your email and we&apos;ll send you a reset link.
        </p>
      </div>

      <form onSubmit={handleRequestReset} className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="email" className="text-foreground">
            Email
          </Label>
          <Input
            id="email"
            type="email"
            placeholder="name@example.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            className="bg-input border-border text-foreground placeholder:text-muted-foreground"
          />
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <Button
          type="submit"
          className="w-full bg-primary text-primary-foreground hover:bg-primary/90"
          disabled={loading}
        >
          {loading ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Sending reset link...
            </>
          ) : (
            "Send reset link"
          )}
        </Button>
      </form>

      <p className="text-center text-sm text-muted-foreground">
        Remembered your password?{" "}
        <Link href="/login" className="font-medium text-primary hover:underline">
          Sign in
        </Link>
      </p>
    </>
  );

  const renderEmailSentState = () => (
    <div className="space-y-6 text-center">
      <div className="flex flex-col items-center space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">
          Check your email
        </h1>
        <p className="text-sm text-muted-foreground">
          If an account exists for{" "}
          <strong className="text-foreground">{email}</strong>, we sent a
          password reset link.
        </p>
      </div>
      <Button
        onClick={() => router.push("/login")}
        variant="outline"
        className="w-full border-border text-foreground hover:bg-secondary"
      >
        Back to login
      </Button>
    </div>
  );

  const renderUpdateForm = () => (
    <>
      <div className="flex flex-col items-center space-y-2 text-center">
        <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-primary">
          <FileText className="h-6 w-6 text-primary-foreground" />
        </div>
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">
          Set a new password
        </h1>
        <p className="text-sm text-muted-foreground">
          Enter your new password to finish resetting your account.
        </p>
      </div>

      <form onSubmit={handleUpdatePassword} className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="password" className="text-foreground">
            New password
          </Label>
          <Input
            id="password"
            type="password"
            placeholder="Enter a new password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            className="bg-input border-border text-foreground placeholder:text-muted-foreground"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="confirmPassword" className="text-foreground">
            Confirm new password
          </Label>
          <Input
            id="confirmPassword"
            type="password"
            placeholder="Re-enter your new password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            required
            className="bg-input border-border text-foreground placeholder:text-muted-foreground"
          />
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <Button
          type="submit"
          className="w-full bg-primary text-primary-foreground hover:bg-primary/90"
          disabled={loading}
        >
          {loading ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Updating password...
            </>
          ) : (
            "Update password"
          )}
        </Button>
      </form>
    </>
  );

  const renderSuccessState = () => (
    <div className="space-y-6 text-center">
      <div className="flex flex-col items-center space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">
          Password updated
        </h1>
        <p className="text-sm text-muted-foreground">
          Your password has been reset. Sign in with your new password.
        </p>
      </div>
      <Button
        onClick={() => router.push("/login")}
        className="w-full bg-primary text-primary-foreground hover:bg-primary/90"
      >
        Go to login
      </Button>
    </div>
  );

  const renderContent = () => {
    if (isCheckingRecovery) {
      return (
        <div className="space-y-4 text-center">
          <Loader2 className="mx-auto h-6 w-6 animate-spin text-muted-foreground" />
          <p className="text-sm text-muted-foreground">
            Checking password reset link...
          </p>
        </div>
      );
    }

    if (mode === "email-sent") {
      return renderEmailSentState();
    }

    if (mode === "update") {
      return renderUpdateForm();
    }

    if (mode === "success") {
      return renderSuccessState();
    }

    return renderRequestForm();
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4">
      <div className="w-full max-w-sm space-y-6">{renderContent()}</div>
    </div>
  );
}
