import { MemoContainer } from "@/components/memo-container";
import { createServerClient } from "@/lib/supabase/server";

export default async function Home() {
  const supabase = await createServerClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  return <MemoContainer initialUser={user ?? null} />;
}
