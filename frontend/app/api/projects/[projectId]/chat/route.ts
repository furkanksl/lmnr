import { coreMessageSchema, streamText } from "ai";
import { and, eq } from "drizzle-orm";
import { z } from "zod";

import { decodeApiKey } from "@/lib/crypto";
import { db } from "@/lib/db/drizzle";
import { providerApiKeys } from "@/lib/db/migrations/schema";
import { Provider, providerToApiKey } from "@/lib/pipeline/types";
import { getProviderInstance } from "@/lib/playground/providersRegistry";

export async function POST(req: Request) {
  try {
    const { messages, model, projectId } = await req.json();

    const parseResult = z.array(coreMessageSchema).min(1).safeParse(messages);

    if (!parseResult.success) {
      throw new Error(`Messages doesn't match structure: ${parseResult.error}`);
    }

    const [provider, modelId] = model.split(":") as [Provider, string];

    const apiKeyName = providerToApiKey[provider];

    const [key] = await db
      .select({
        value: providerApiKeys.value,
        nonceHex: providerApiKeys.nonceHex,
        name: providerApiKeys.name,
        createdAt: providerApiKeys.createdAt,
      })
      .from(providerApiKeys)
      .where(and(eq(providerApiKeys.projectId, projectId), eq(providerApiKeys.name, apiKeyName)));

    if (!key) {
      throw new Error("No matching key found.");
    }

    const decodedKey = await decodeApiKey(key.name, key.nonceHex, key.value);
    const providerOptions: Record<string, any> = {};

    if (provider === "anthropic" && modelId === "claude-3-7-sonnet-20250219-thinking") {
      providerOptions["anthropic"] = {
        thinking: {
          type: "enabled",
          budgetTokens: 25000,
        },
      };
    }

    const providerInstance = getProviderInstance(model, decodedKey);

    const result = streamText({
      model: providerInstance,
      messages,
      maxTokens: 30000,
      temperature: 1,
      providerOptions
    });

    return result.toTextStreamResponse();
  } catch (e) {
    return new Response(
      JSON.stringify({
        error: e instanceof Error ? e.message : "Internal server error.",
        details: e instanceof Error ? e.name : "Unknown error",
      }),
      {
        status: 500,
      }
    );
  }
}
