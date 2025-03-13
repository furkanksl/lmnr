import {eq} from "drizzle-orm";
import { Metadata } from "next";
import { redirect } from "next/navigation";
import { getServerSession } from "next-auth";

import NotFound from "@/app/not-found";
import Chat from "@/components/chat";
import { authOptions } from "@/lib/auth";
import {db} from "@/lib/db/drizzle";
import {users} from "@/lib/db/migrations/schema";

export const metadata: Metadata = {
  title: "Agent",
};

export default async function ChatPage() {
  const session = await getServerSession(authOptions);

  const chatId = crypto.randomUUID();
  if (!session) {
    redirect("/sign-in?callbackUrl=/onboarding");
  }

  const user = session.user;

  const result = await db.query.users.findFirst({
    where: eq(users.email, String(user.email)),
    columns: {
      id: true,
    }
  });

  if (!result) {
    return <NotFound />;
  }

  return <Chat chatId={chatId} userId={result.id} initialMessages={[]} />;
}
