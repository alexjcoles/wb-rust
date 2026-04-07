import type { AnalyzeResponse, ChatResponse, ParseResponse } from "../types/build";

const BASE_URL = "http://localhost:5656/api";

export async function parseBuild(url: string): Promise<ParseResponse> {
  const res = await fetch(`${BASE_URL}/parse`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ url }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function analyzeBuild(url: string): Promise<AnalyzeResponse> {
  const res = await fetch(`${BASE_URL}/analyze`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ url }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function chat(
  messages: { role: string; content: string }[],
  provider = "claude-cli",
  model?: string
): Promise<ChatResponse> {
  const res = await fetch(`${BASE_URL}/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ messages, provider, model }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}
