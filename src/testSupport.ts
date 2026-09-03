export const isTestMode = import.meta.env.VITE_TOKENGLASS_TEST_MODE === "true";

export const sampleUsage = {
  totalBilled: 12.34,
  todayUsage: 0.56,
  currency: "USD",
  inputTokens: 125_000,
  outputTokens: 32_000,
  models: [
    { name: "gpt-4.1", tokens: 92_000 },
    { name: "gpt-4o-mini", tokens: 65_000 },
  ],
  periodStart: 0,
  periodEnd: 0,
};

export function redactDiagnosticText(value: string): string {
  return value
    .replace(/\bsk-[A-Za-z0-9_-]+/g, "[redacted]")
    .replace(/\bBearer\s+[^\s,;]+/gi, "Bearer [redacted]")
    .replace(
      /(access[_-]?token|refresh[_-]?token|authorization)\s*[:=]\s*[^\s,;]+/gi,
      "$1=[redacted]",
    );
}
