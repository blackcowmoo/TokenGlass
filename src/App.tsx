import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import "./App.css";
import { isTestMode, redactDiagnosticText, sampleUsage } from "./testSupport";
import {
  formatExchangeRate,
  formatLastSuccess,
  formatKrwReference,
  formatOriginalCost,
  USAGE_REFRESH_INTERVAL_MS,
  usdToKrwRate,
  validUsdToKrwRate,
  type Usage,
  type UsageSnapshot,
} from "./usage";

type SubscriptionUsage = {
  email?: string;
  planType?: string;
  limits: {
    limitId: string;
    label?: string;
    usedPercent?: number;
    windowDurationMins?: number;
    resetsAt?: number;
  }[];
  lifetimeTokens?: number;
  peakDailyTokens?: number;
  dailyUsage: { startDate: string; tokens: number }[];
};

type RuntimeDiagnostics = {
  appVersion: string;
  operatingSystem: string;
  architecture: string;
  testMode: boolean;
  sidecarAvailable: boolean;
  sidecarRunning: boolean;
};

const colors = ["#10A37F", "#3B82F6", "#A855F7", "#F59E0B", "#EC4899"];

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [adminKey, setAdminKey] = useState("");
  const [exchangeRate, setExchangeRate] = useState(usdToKrwRate(undefined));
  const [exchangeRateInput, setExchangeRateInput] = useState(String(usdToKrwRate(undefined)));
  const [exchangeRateError, setExchangeRateError] = useState<string | null>(null);
  const [usage, setUsage] = useState<Usage | null>(null);
  const [status, setStatus] = useState("OpenAI 조직 관리자 API 키를 연결하세요.");
  const [loading, setLoading] = useState(false);
  const [subscription, setSubscription] = useState<SubscriptionUsage | null>(null);
  const [subscriptionStatus, setSubscriptionStatus] = useState(
    "ChatGPT OAuth 로그인을 연결하세요.",
  );
  const [subscriptionLoading, setSubscriptionLoading] = useState(false);
  const [diagnostics, setDiagnostics] = useState<string | null>(null);
  const adminKeyRef = useRef("");
  const usageRef = useRef<Usage | null>(null);
  const loadingRef = useRef(false);

  const refresh = useCallback(async (key = adminKeyRef.current, forceRefresh = false) => {
    if (isTestMode) {
      setUsage(sampleUsage);
      setStatus("TEST MODE · Sample data · network disabled");
      return;
    }
    if (!key.trim()) return;
    if (loadingRef.current) return;
    loadingRef.current = true;
    setLoading(true);
    setStatus(
      usageRef.current ? "OpenAI 사용량을 업데이트하는 중…" : "OpenAI 사용량을 불러오는 중…",
    );
    try {
      const snapshot = await invoke<UsageSnapshot>("fetch_openai_usage", {
        adminKey: key,
        forceRefresh,
      });
      usageRef.current = snapshot.usage;
      setUsage(snapshot.usage);
      const lastSuccess = formatLastSuccess(snapshot.fetchedAt);
      if (snapshot.stale) {
        setStatus(
          `업데이트 실패 · ${lastSuccess} 데이터 표시 중${snapshot.refreshError ? ` · ${snapshot.refreshError}` : ""}`,
        );
      } else {
        setStatus(`마지막 성공: ${lastSuccess}${snapshot.source === "cache" ? " · 캐시" : ""}`);
      }
    } catch (error) {
      if (!usageRef.current) setUsage(null);
      setStatus(String(error));
    } finally {
      loadingRef.current = false;
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void (async () => {
      try {
        if (isTestMode) {
          setUsage(sampleUsage);
          setStatus("TEST MODE · Sample data · network disabled");
          return;
        }
        if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
          const store = await Store.load("settings.json");
          const key = (await store.get<string>("tokenglass_openai_admin_key")) ?? "";
          const savedExchangeRate = await store.get<unknown>("tokenglass_usd_to_krw_rate");
          const nextExchangeRate = usdToKrwRate(savedExchangeRate);
          setAdminKey(key);
          setExchangeRate(nextExchangeRate);
          setExchangeRateInput(String(nextExchangeRate));
          adminKeyRef.current = key;
          if (key) await refresh(key);
        } else {
          console.warn("Tauri 환경이 아닙니다. 웹 브라우저 모드로 동작합니다.");
        }
      } catch (error) {
        console.error("설정을 불러오는 중 오류가 발생했습니다:", error);
      }
    })();
  }, [refresh]);

  useEffect(() => {
    if (isTestMode) return;
    const interval = window.setInterval(() => void refresh(), USAGE_REFRESH_INTERVAL_MS);
    const refreshWhenActive = () => void refresh();
    window.addEventListener("focus", refreshWhenActive);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener("focus", refreshWhenActive);
    };
  }, [refresh]);

  const saveSettings = async () => {
    if (isTestMode) return;
    const nextKey = adminKey.trim();
    const nextExchangeRate = validUsdToKrwRate(exchangeRateInput);
    if (nextExchangeRate === null) {
      setExchangeRateError("환율은 0보다 큰 숫자로 입력하세요.");
      return;
    }
    const keyChanged = nextKey !== adminKeyRef.current;
    const store = await Store.load("settings.json");
    await store.set("tokenglass_openai_admin_key", nextKey);
    await store.set("tokenglass_usd_to_krw_rate", nextExchangeRate);
    await store.save();
    adminKeyRef.current = nextKey;
    setExchangeRate(nextExchangeRate);
    setExchangeRateInput(String(nextExchangeRate));
    setExchangeRateError(null);
    setShowSettings(false);
    if (keyChanged) {
      usageRef.current = null;
      setUsage(null);
    }
    if (nextKey) await refresh(nextKey, true);
    else setStatus("OpenAI 조직 관리자 API 키를 연결하세요.");
  };

  const refreshSubscription = async () => {
    if (isTestMode) {
      setSubscriptionStatus("TEST MODE에서는 ChatGPT OAuth를 사용하지 않습니다.");
      return;
    }
    setSubscriptionLoading(true);
    setSubscriptionStatus("ChatGPT 구독 사용량을 불러오는 중…");
    try {
      const result = await invoke<SubscriptionUsage>("fetch_chatgpt_subscription_usage");
      setSubscription(result);
      setSubscriptionStatus(`마지막 동기화: ${new Date().toLocaleTimeString()}`);
    } catch (error) {
      setSubscription(null);
      setSubscriptionStatus(String(error));
    } finally {
      setSubscriptionLoading(false);
    }
  };

  const connectChatGpt = async () => {
    if (isTestMode) {
      setSubscriptionStatus("TEST MODE에서는 ChatGPT OAuth를 사용하지 않습니다.");
      return;
    }
    setSubscriptionLoading(true);
    try {
      const login = await invoke<{ authUrl: string }>("start_chatgpt_login");
      setSubscriptionStatus("브라우저에서 로그인을 완료한 후 ‘구독 사용량 새로고침’을 누르세요.");
      window.open(login.authUrl, "_blank");
    } catch (error) {
      setSubscriptionStatus(String(error));
    } finally {
      setSubscriptionLoading(false);
    }
  };

  const maxTokens = useMemo(
    () => Math.max(...(usage?.models.map((model) => model.tokens) ?? [1]), 1),
    [usage],
  );
  const monthName = new Intl.DateTimeFormat(undefined, { month: "long" }).format(new Date());

  const showDiagnostics = async () => {
    try {
      const runtime = await invoke<RuntimeDiagnostics>("get_runtime_diagnostics");
      const storeState = isTestMode ? "not used in test mode" : "available";
      setDiagnostics(
        redactDiagnosticText(
          [
            `TokenGlass ${runtime.appVersion}`,
            `OS: ${runtime.operatingSystem}/${runtime.architecture}`,
            `Mode: ${runtime.testMode ? "test" : "standard"}`,
            `Codex sidecar: ${runtime.sidecarAvailable ? "available" : "missing"}; ${runtime.sidecarRunning ? "running" : "not started"}`,
            `Settings store: ${storeState}`,
          ].join("\n"),
        ),
      );
    } catch (error) {
      setDiagnostics(redactDiagnosticText(`Diagnostics unavailable: ${String(error)}`));
    }
  };

  return (
    <main className="app-shell">
      {showSettings && (
        <div className="modal-overlay">
          <div className="glass-panel modal-content">
            <div className="modal-header">
              <h3>OpenAI 연결</h3>
              <button className="icon-btn" onClick={() => setShowSettings(false)}>
                ✕
              </button>
            </div>
            <div className="modal-body settings-body">
              <label htmlFor="openai-admin-key">OpenAI 조직 관리자 API 키</label>
              <input
                id="openai-admin-key"
                type="password"
                className="test-textarea glass-panel compact-input"
                placeholder="sk-admin-..."
                value={adminKey}
                onChange={(event) => setAdminKey(event.target.value)}
              />
              <p className="help-text">
                Usage/Costs API는 일반 프로젝트 키가 아닌 조직 관리자 키가 필요합니다.
              </p>
              <label htmlFor="usd-to-krw-rate">수동 환율 (1 USD당 KRW)</label>
              <input
                id="usd-to-krw-rate"
                type="number"
                min="0"
                step="0.01"
                className="test-textarea glass-panel compact-input"
                value={exchangeRateInput}
                onChange={(event) => {
                  setExchangeRateInput(event.target.value);
                  setExchangeRateError(null);
                }}
              />
              <p className="help-text">환율 API를 호출하지 않으며, KRW 금액은 참고용입니다.</p>
              {exchangeRateError && <p className="settings-error">{exchangeRateError}</p>}
              <div className="subscription-note">
                <strong>ChatGPT/Codex 구독 OAuth</strong>
                <br />
                OpenAI는 OAuth 로그인으로 Plus/Pro/Codex의 사용량 또는 남은 한도를 읽는 공개 API를
                제공하지 않습니다. 구독과 API 과금은 별도입니다.
              </div>
            </div>
            <div className="modal-footer">
              <button
                className="primary-btn"
                disabled={loading}
                onClick={() => void saveSettings()}
              >
                저장 및 동기화
              </button>
            </div>
          </div>
        </div>
      )}

      {diagnostics && (
        <div className="modal-overlay">
          <div className="glass-panel modal-content">
            <div className="modal-header">
              <h3>진단 정보</h3>
              <button className="icon-btn" onClick={() => setDiagnostics(null)}>
                ✕
              </button>
            </div>
            <pre className="diagnostics-output">{diagnostics}</pre>
            <div className="modal-footer">
              <button
                className="primary-btn"
                onClick={() => void navigator.clipboard?.writeText(diagnostics)}
              >
                복사
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="glass-panel widget-container">
        <div className="widget-header">
          <div className="widget-title">
            OpenAI Usage Dashboard{" "}
            {isTestMode && <span className="test-mode-badge">TEST MODE</span>}
          </div>
          <div className="header-actions">
            <button
              className="icon-btn"
              title="새로고침"
              disabled={loading || (!adminKey && !isTestMode)}
              onClick={() => void refresh(undefined, true)}
            >
              ↻
            </button>
            <button className="icon-btn" title="진단 정보" onClick={() => void showDiagnostics()}>
              ⓘ
            </button>
            <button
              className="icon-btn"
              title="연결 설정"
              disabled={isTestMode}
              onClick={() => setShowSettings(true)}
            >
              ⚙
            </button>
          </div>
        </div>
        <div className="usage-source">
          <span className={usage ? "status-dot connected" : "status-dot"} />{" "}
          {isTestMode
            ? "Sample data · no network"
            : `API key usage ${usage ? "connected" : "not connected"}`}
        </div>
        <div className="total-section">
          <div className="section-caption" title="시스템 로컬 시간대 기준 당월 지출액">
            Current Month ({monthName})
          </div>
          <div className="total-cost">
            {usage ? formatOriginalCost(usage.totalBilled, usage.currency) : "—"}
          </div>
          {usage && formatKrwReference(usage.totalBilled, usage.currency, exchangeRate) && (
            <div className="cost-reference">
              {formatKrwReference(usage.totalBilled, usage.currency, exchangeRate)} ·{" "}
              {formatExchangeRate(exchangeRate)}
            </div>
          )}
          <div className="sync-status">{status}</div>
        </div>
        <div className="models-breakdown">
          <div className="section-caption">TOKEN USAGE BY MODEL</div>
          {usage?.models.length ? (
            usage.models.slice(0, 5).map((model, index) => (
              <div key={model.name} className="model-row">
                <div className="model-info">
                  <span className="model-name">{model.name}</span>
                  <span className="model-cost">{model.tokens.toLocaleString()} tokens</span>
                </div>
                <div className="model-bar-bg">
                  <div
                    className="model-bar-fill"
                    style={{
                      width: `${(model.tokens / maxTokens) * 100}%`,
                      backgroundColor: colors[index % colors.length],
                    }}
                  />
                </div>
              </div>
            ))
          ) : (
            <div className="empty-state">연결 후 모델별 토큰 사용량이 표시됩니다.</div>
          )}
        </div>
        <div className="daily-progress">
          <div className="daily-info">
            <span title="시스템 로컬 시간대 자정(00:00) 기준 오늘 지출액">
              Today’s API spending (Local)
            </span>
            <span>{usage ? formatOriginalCost(usage.todayUsage, usage.currency) : "—"}</span>
          </div>
          {usage && formatKrwReference(usage.todayUsage, usage.currency, exchangeRate) && (
            <div className="cost-reference daily-cost-reference">
              {formatKrwReference(usage.todayUsage, usage.currency, exchangeRate)} ·{" "}
              {formatExchangeRate(exchangeRate)}
            </div>
          )}
          {usage && (
            <div className="token-summary">
              <span>Input {usage.inputTokens.toLocaleString()}</span>
              <span>Output {usage.outputTokens.toLocaleString()}</span>
            </div>
          )}
        </div>
        <div className="subscription-card subscription-panel">
          <div className="subscription-heading">
            <div>
              <strong>ChatGPT/Codex subscription</strong>
              <p>
                {subscription
                  ? `${subscription.planType ?? "ChatGPT"} · ${subscription.email ?? "연결됨"}`
                  : subscriptionStatus}
              </p>
            </div>
            <div className="subscription-actions">
              <button
                className="secondary-btn"
                disabled={subscriptionLoading || isTestMode}
                onClick={() => void connectChatGpt()}
              >
                ChatGPT 로그인
              </button>
              <button
                className="secondary-btn"
                disabled={subscriptionLoading || isTestMode}
                onClick={() => void refreshSubscription()}
              >
                ↻
              </button>
            </div>
          </div>
          {subscription?.limits.length ? (
            <div className="limit-list">
              {subscription.limits.map((limit) => (
                <div className="limit-row" key={limit.limitId}>
                  <div className="daily-info">
                    <span>
                      {limit.label || limit.limitId}{" "}
                      {limit.windowDurationMins ? `(${limit.windowDurationMins}m)` : ""}
                    </span>
                    <span>{limit.usedPercent?.toFixed(0) ?? "—"}%</span>
                  </div>
                  <div className="budget-bar">
                    <div
                      className="budget-fill"
                      style={{
                        width: `${Math.min(limit.usedPercent ?? 0, 100)}%`,
                        backgroundColor: "var(--color-accent)",
                      }}
                    />
                  </div>
                  {limit.resetsAt && (
                    <span className="reset-time">
                      Resets {new Date(limit.resetsAt * 1000).toLocaleString()}
                    </span>
                  )}
                </div>
              ))}
            </div>
          ) : null}
          {subscription && (
            <div className="token-summary">
              <span>Lifetime {subscription.lifetimeTokens?.toLocaleString() ?? "—"} tokens</span>
              <span>Peak daily {subscription.peakDailyTokens?.toLocaleString() ?? "—"}</span>
            </div>
          )}
        </div>
      </div>
    </main>
  );
}

export default App;
