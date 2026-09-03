import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import "./App.css";
import { isTestMode, sampleUsage } from "./testSupport";
import {
  formatExchangeRate,
  formatKrwReference,
  formatOriginalCost,
  USAGE_REFRESH_INTERVAL_MS,
  usdToKrwRate,
  type Usage,
  type UsageSnapshot,
} from "./usage";

export default function Widget() {
  const [usage, setUsage] = useState<Usage | null>(null);
  const [exchangeRate, setExchangeRate] = useState(usdToKrwRate(undefined));
  const [refreshError, setRefreshError] = useState<string | null>(null);

  useEffect(() => {
    let interval: number;

    const fetchUsage = async () => {
      if (isTestMode) {
        setUsage(sampleUsage);
        return;
      }
      try {
        if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
          const store = await Store.load("settings.json");
          const key = (await store.get<string>("tokenglass_openai_admin_key")) ?? "";
          setExchangeRate(usdToKrwRate(await store.get<unknown>("tokenglass_usd_to_krw_rate")));
          if (key) {
            const snapshot = await invoke<UsageSnapshot>("fetch_openai_usage", {
              adminKey: key,
              forceRefresh: false,
            });
            setUsage(snapshot.usage);
            setRefreshError(snapshot.stale ? (snapshot.refreshError ?? "업데이트 실패") : null);
          }
        }
      } catch (error) {
        setRefreshError(String(error));
        console.error("위젯 사용량 조회 오류:", error);
      }
    };

    void fetchUsage();
    interval = window.setInterval(() => void fetchUsage(), USAGE_REFRESH_INTERVAL_MS);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="widget-root" data-tauri-drag-region>
      <div className="widget-content" data-tauri-drag-region title={refreshError ?? undefined}>
        <span className="widget-label" data-tauri-drag-region>
          {isTestMode ? "Test · Today" : refreshError ? "Today · update failed" : "Today"}
        </span>
        <span className="widget-value" data-tauri-drag-region>
          {usage ? formatOriginalCost(usage.todayUsage, usage.currency) : "—"}
        </span>
        {usage && formatKrwReference(usage.todayUsage, usage.currency, exchangeRate) && (
          <span className="widget-reference" data-tauri-drag-region>
            {formatKrwReference(usage.todayUsage, usage.currency, exchangeRate)} ·{" "}
            {formatExchangeRate(exchangeRate)}
          </span>
        )}
      </div>
    </div>
  );
}
