import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import "./App.css";

type Usage = {
  totalBilled: number;
  todayUsage: number;
  inputTokens: number;
  outputTokens: number;
  models: { name: string; tokens: number }[];
  periodStart: number;
  periodEnd: number;
};

type SubscriptionUsage = {
  email?: string;
  planType?: string;
  limits: { limitId: string; label?: string; usedPercent?: number; windowDurationMins?: number; resetsAt?: number }[];
  lifetimeTokens?: number;
  peakDailyTokens?: number;
  dailyUsage: { startDate: string; tokens: number }[];
};

const colors = ["#10A37F", "#3B82F6", "#A855F7", "#F59E0B", "#EC4899"];

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [adminKey, setAdminKey] = useState("");
  const [usage, setUsage] = useState<Usage | null>(null);
  const [status, setStatus] = useState("OpenAI 조직 관리자 API 키를 연결하세요.");
  const [loading, setLoading] = useState(false);
  const [subscription, setSubscription] = useState<SubscriptionUsage | null>(null);
  const [subscriptionStatus, setSubscriptionStatus] = useState("ChatGPT OAuth 로그인을 연결하세요.");
  const [subscriptionLoading, setSubscriptionLoading] = useState(false);

  const refresh = async (key = adminKey) => {
    if (!key.trim()) return;
    setLoading(true);
    setStatus("OpenAI 사용량을 불러오는 중…");
    try {
      const result = await invoke<Usage>("fetch_openai_usage", { adminKey: key });
      setUsage(result);
      setStatus(`마지막 동기화: ${new Date().toLocaleTimeString()}`);
    } catch (error) {
      setUsage(null);
      setStatus(String(error));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void (async () => {
      try {
        if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
          const store = await Store.load("settings.json");
          const key = (await store.get<string>("tokenglass_openai_admin_key")) ?? "";
          setAdminKey(key);
          if (key) await refresh(key);
        } else {
          console.warn("Tauri 환경이 아닙니다. 웹 브라우저 모드로 동작합니다.");
        }
      } catch (error) {
        console.error("설정을 불러오는 중 오류가 발생했습니다:", error);
      }
    })();
  }, []);

  const saveSettings = async () => {
    const store = await Store.load("settings.json");
    await store.set("tokenglass_openai_admin_key", adminKey.trim());
    await store.save();
    setShowSettings(false);
    await refresh(adminKey);
  };

  const refreshSubscription = async () => {
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

  const maxTokens = useMemo(() => Math.max(...(usage?.models.map((model) => model.tokens) ?? [1]), 1), [usage]);
  const monthName = new Intl.DateTimeFormat(undefined, { month: "long" }).format(new Date());

  return (
    <main className="app-shell">
      {showSettings && <div className="modal-overlay">
        <div className="glass-panel modal-content">
          <div className="modal-header"><h3>OpenAI 연결</h3><button className="icon-btn" onClick={() => setShowSettings(false)}>✕</button></div>
          <div className="modal-body settings-body">
            <label htmlFor="openai-admin-key">OpenAI 조직 관리자 API 키</label>
            <input id="openai-admin-key" type="password" className="test-textarea glass-panel compact-input" placeholder="sk-admin-..." value={adminKey} onChange={(event) => setAdminKey(event.target.value)} />
            <p className="help-text">Usage/Costs API는 일반 프로젝트 키가 아닌 조직 관리자 키가 필요합니다.</p>
            <div className="subscription-note"><strong>ChatGPT/Codex 구독 OAuth</strong><br />OpenAI는 OAuth 로그인으로 Plus/Pro/Codex의 사용량 또는 남은 한도를 읽는 공개 API를 제공하지 않습니다. 구독과 API 과금은 별도입니다.</div>
          </div>
          <div className="modal-footer"><button className="primary-btn" onClick={() => void saveSettings()}>저장 및 동기화</button></div>
        </div>
      </div>}

      <div className="glass-panel widget-container">
        <div className="widget-header">
          <div className="widget-title">OpenAI Usage Dashboard</div>
          <div className="header-actions"><button className="icon-btn" title="새로고침" disabled={loading || !adminKey} onClick={() => void refresh()}>↻</button><button className="icon-btn" title="연결 설정" onClick={() => setShowSettings(true)}>⚙</button></div>
        </div>
        <div className="usage-source"><span className={usage ? "status-dot connected" : "status-dot"} /> API key usage {usage ? "connected" : "not connected"}</div>
        <div className="total-section">
          <div className="section-caption">Current Month ({monthName})</div>
          <div className="total-cost">{usage ? `$${usage.totalBilled.toFixed(2)}` : "—"}</div>
          <div className="sync-status">{status}</div>
        </div>
        <div className="models-breakdown">
          <div className="section-caption">TOKEN USAGE BY MODEL</div>
          {usage?.models.length ? usage.models.slice(0, 5).map((model, index) => <div key={model.name} className="model-row">
            <div className="model-info"><span className="model-name">{model.name}</span><span className="model-cost">{model.tokens.toLocaleString()} tokens</span></div>
            <div className="model-bar-bg"><div className="model-bar-fill" style={{ width: `${(model.tokens / maxTokens) * 100}%`, backgroundColor: colors[index % colors.length] }} /></div>
          </div>) : <div className="empty-state">연결 후 모델별 토큰 사용량이 표시됩니다.</div>}
        </div>
        <div className="daily-progress">
          <div className="daily-info"><span>Today’s API spending</span><span>{usage ? `$${usage.todayUsage.toFixed(2)}` : "—"}</span></div>
          {usage && <div className="token-summary"><span>Input {usage.inputTokens.toLocaleString()}</span><span>Output {usage.outputTokens.toLocaleString()}</span></div>}
        </div>
        <div className="subscription-card subscription-panel">
          <div className="subscription-heading"><div><strong>ChatGPT/Codex subscription</strong><p>{subscription ? `${subscription.planType ?? "ChatGPT"} · ${subscription.email ?? "연결됨"}` : subscriptionStatus}</p></div><div className="subscription-actions"><button className="secondary-btn" disabled={subscriptionLoading} onClick={() => void connectChatGpt()}>ChatGPT 로그인</button><button className="secondary-btn" disabled={subscriptionLoading} onClick={() => void refreshSubscription()}>↻</button></div></div>
          {subscription?.limits.length ? <div className="limit-list">{subscription.limits.map((limit) => <div className="limit-row" key={limit.limitId}><div className="daily-info"><span>{limit.label || limit.limitId} {limit.windowDurationMins ? `(${limit.windowDurationMins}m)` : ""}</span><span>{limit.usedPercent?.toFixed(0) ?? "—"}%</span></div><div className="budget-bar"><div className="budget-fill" style={{ width: `${Math.min(limit.usedPercent ?? 0, 100)}%`, backgroundColor: "var(--color-accent)" }} /></div>{limit.resetsAt && <span className="reset-time">Resets {new Date(limit.resetsAt * 1000).toLocaleString()}</span>}</div>)}</div> : null}
          {subscription && <div className="token-summary"><span>Lifetime {subscription.lifetimeTokens?.toLocaleString() ?? "—"} tokens</span><span>Peak daily {subscription.peakDailyTokens?.toLocaleString() ?? "—"}</span></div>}
        </div>
      </div>
    </main>
  );
}

export default App;
