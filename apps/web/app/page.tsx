import { MemoContainer } from "@/components/memo-container";

export const dynamic = "force-static";

export default function Home() {
  return <MemoContainer initialUser={null} />;
}
