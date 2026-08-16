export type Usage = {
  totalBilled: number;
  todayUsage: number;
  inputTokens: number;
  outputTokens: number;
  models: { name: string; tokens: number }[];
  periodStart: number;
  periodEnd: number;
};

export type UsageSnapshot = {
  usage: Usage;
  fetchedAt: number;
  source: "network" | "cache";
  stale: boolean;
  refreshError?: string;
};

export const USAGE_REFRESH_INTERVAL_MS = 5 * 60 * 1000;

export function formatLastSuccess(fetchedAt: number): string {
  return new Date(fetchedAt * 1000).toLocaleTimeString();
}
