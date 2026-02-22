import { useCallback, useEffect, useState } from "react";
import {
  type AuthChangeEvent,
  type AuthUser as User,
  type Session,
  type SupabaseClient,
} from "@supabase/supabase-js";

export function useResolvedUser(
  initialUser: User | null,
  supabase: SupabaseClient,
) {
  const [resolvedUser, setResolvedUser] = useState<User | null>(initialUser);

  useEffect(() => {
    let isCancelled = false;

    const resolveSession = async () => {
      if (initialUser) {
        setResolvedUser(initialUser);
        return;
      }

      try {
        const { data, error } = await supabase.auth.getSession();
        if (isCancelled) return;
        setResolvedUser(
          error || !data?.session?.user ? null : data.session.user,
        );
      } catch {
        if (!isCancelled) {
          setResolvedUser(null);
        }
      }
    };

    resolveSession();
    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange(
      (_event: AuthChangeEvent, session: Session | null) => {
        if (!isCancelled) {
          setResolvedUser(session?.user ?? null);
        }
      },
    );

    return () => {
      isCancelled = true;
      subscription.unsubscribe();
    };
  }, [initialUser, supabase]);

  const resolveSubmitUser = useCallback(async (): Promise<User | null> => {
    if (resolvedUser) return resolvedUser;
    if (initialUser) return initialUser;
    try {
      const { data, error } = await supabase.auth.getSession();
      if (error || !data?.session?.user) return null;
      return data.session.user;
    } catch {
      return null;
    }
  }, [initialUser, resolvedUser, supabase]);

  return { resolvedUser, resolveSubmitUser };
}
