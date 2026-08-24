export type Usage = {
  totalBilled: number;
  todayUsage: number;
  currency: string;
  inputTokens: number;
  outputTokens: number;
  models: { name: string; tokens: number }[];
  periodStart: number;
  periodEnd: number;
};

export const DEFAULT_USD_TO_KRW_RATE = 1350;

const formatters = new Map<string, Intl.NumberFormat>();

function currencyFormatter(currency: string): Intl.NumberFormat {
  const normalizedCurrency = currency.toUpperCase();
  const cached = formatters.get(normalizedCurrency);
  if (cached) return cached;
  const formatter = new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: normalizedCurrency,
  });
  formatters.set(normalizedCurrency, formatter);
  return formatter;
}

export function validUsdToKrwRate(value: unknown): number | null {
  const rate = typeof value === "number" ? value : Number(value);
  return Number.isFinite(rate) && rate > 0 ? rate : null;
}

export function usdToKrwRate(value: unknown): number {
  return validUsdToKrwRate(value) ?? DEFAULT_USD_TO_KRW_RATE;
}

export function formatOriginalCost(amount: number, currency: string): string {
  try {
    return currencyFormatter(currency).format(amount);
  } catch {
    return `${amount.toFixed(2)} ${currency.toUpperCase()}`;
  }
}

export function formatKrwReference(amount: number, currency: string, rate: number): string | null {
  if (currency.toUpperCase() !== "USD") return null;
  return new Intl.NumberFormat("ko-KR", {
    style: "currency",
    currency: "KRW",
    maximumFractionDigits: 0,
  }).format(amount * usdToKrwRate(rate));
}

export function formatExchangeRate(rate: number): string {
  return `1 USD = ${new Intl.NumberFormat("ko-KR", {
    maximumFractionDigits: 2,
  }).format(usdToKrwRate(rate))} KRW`;
}

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
