import { useState, useEffect } from "react";
import { encodingForModel } from "js-tiktoken";
import { Store } from "@tauri-apps/plugin-store";
import "./App.css";

// Mock Data for Dashboard
const MOCK_USAGE_DATA = {
  totalBilled: 12.45,
  dailyBudget: 2.00,
  todayUsage: 1.65,
  models: [
    { name: "GPT-4o", provider: "OpenAI", cost: 8.20, color: "#10A37F" },
    { name: "Claude 3.5 Sonnet", provider: "Anthropic", cost: 3.15, color: "#D97757" },
    { name: "Gemini 1.5 Pro", provider: "Google", cost: 1.10, color: "#4285F4" },
  ]
};

// Initialize secure store
const store = new Store('settings.json');

function App() {
  const [showSettings, setShowSettings] = useState(false);

  // Settings States
  const [keys, setKeys] = useState({ openai: "", anthropic: "", google: "" });

  // Clipboard Toast Mock State
  const [showToast, setShowToast] = useState(false);
  const [toastData, setToastData] = useState({ tokens: 0, cost: 0, model: "GPT-4o" });
  const [inputText, setInputText] = useState("");

  // Load API Keys from secure store on mount
  useEffect(() => {
    async function loadKeys() {
      const openai = await store.get<string>("tokenglass_openai_key") || "";
      const anthropic = await store.get<string>("tokenglass_anthropic_key") || "";
      const google = await store.get<string>("tokenglass_google_key") || "";
      setKeys({ openai, anthropic, google });
    }
    loadKeys();
  }, []);

  const saveSettings = async () => {
    await store.set("tokenglass_openai_key", keys.openai);
    await store.set("tokenglass_anthropic_key", keys.anthropic);
    await store.set("tokenglass_google_key", keys.google);
    await store.save(); // explicitly save to disk
    setShowSettings(false);
  };

  // Simulate clipboard event when typing in the test area
  useEffect(() => {
    if (!inputText) return;

    try {
      const enc = encodingForModel("gpt-4o");
      const tokenCount = enc.encode(inputText).length;
      const estimatedCost = (tokenCount / 1000000) * 5.00;

      setToastData({ tokens: tokenCount, cost: estimatedCost, model: "GPT-4o" });
      setShowToast(true);

      // Auto-hide toast after 3 seconds
      const timer = setTimeout(() => {
        setShowToast(false);
      }, 3000);
      return () => clearTimeout(timer);
    } catch (e) {
      console.error(e);
    }
  }, [inputText]);

  // Calculate max cost for the bar charts
  const maxModelCost = Math.max(...MOCK_USAGE_DATA.models.map(m => m.cost));

  return (
    <main style={{ minHeight: "100vh", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "32px", padding: "20px" }}>

      {/* Settings Modal */}
      {showSettings && (
        <div className="modal-overlay">
          <div className="glass-panel modal-content">
            <div className="modal-header">
              <h3>API Connections</h3>
              <button className="icon-btn" onClick={() => setShowSettings(false)}>✕</button>
            </div>
            <div className="modal-body" style={{ gap: '16px', display: 'flex', flexDirection: 'column' }}>
              <div>
                <label>OpenAI API Key</label>
                <input
                  type="password" className="test-textarea glass-panel" style={{ height: "36px", padding: "8px", marginTop: "4px" }}
                  placeholder="sk-..." value={keys.openai} onChange={(e) => setKeys({ ...keys, openai: e.target.value })}
                />
              </div>
              <div>
                <label>Anthropic API Key</label>
                <input
                  type="password" className="test-textarea glass-panel" style={{ height: "36px", padding: "8px", marginTop: "4px" }}
                  placeholder="sk-ant-..." value={keys.anthropic} onChange={(e) => setKeys({ ...keys, anthropic: e.target.value })}
                />
              </div>
              <div>
                <label>Google Gemini API Key</label>
                <input
                  type="password" className="test-textarea glass-panel" style={{ height: "36px", padding: "8px", marginTop: "4px" }}
                  placeholder="AIzaSy..." value={keys.google} onChange={(e) => setKeys({ ...keys, google: e.target.value })}
                />
              </div>
              <p className="help-text">Keys are stored securely in your local system keychain.</p>
            </div>
            <div className="modal-footer">
              <button className="primary-btn" onClick={saveSettings}>Save Keys</button>
            </div>
          </div>
        </div>
      )}

      {/* Main Dashboard Widget (Always Visible) */}
      <div className="glass-panel widget-container" style={{ width: "340px" }}>

        {/* Header */}
        <div className="widget-header">
          <div className="widget-title">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
              <line x1="3" y1="9" x2="21" y2="9"></line>
              <line x1="9" y1="21" x2="9" y2="9"></line>
            </svg>
            Usage Dashboard
          </div>
          <button className="icon-btn" onClick={() => setShowSettings(true)}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>
          </button>
        </div>

        {/* Total Billed Section */}
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', margin: '8px 0 16px 0' }}>
          <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-secondary)', fontWeight: 500, marginBottom: '4px' }}>
            Current Month (Oct)
          </div>
          <div style={{ fontSize: '2.5rem', fontWeight: 800, letterSpacing: '-0.03em', lineHeight: 1 }}>
            ${MOCK_USAGE_DATA.totalBilled.toFixed(2)}
          </div>
          <div style={{ fontSize: '0.7rem', color: 'rgba(255, 255, 255, 0.5)', marginTop: '6px' }}>
            Resets in 11d 04h
          </div>
        </div>

        {/* Model Breakdown Graphs */}
        <div className="models-breakdown">
          <div style={{ fontSize: 'var(--text-xs)', fontWeight: 600, color: 'var(--color-text-secondary)', marginBottom: '12px' }}>MODEL USAGE</div>

          {MOCK_USAGE_DATA.models.map((model) => (
            <div key={model.name} className="model-row">
              <div className="model-info">
                <span className="model-name">{model.name}</span>
                <span className="model-cost">${model.cost.toFixed(2)}</span>
              </div>
              <div className="model-bar-bg">
                <div
                  className="model-bar-fill"
                  style={{
                    width: `${(model.cost / maxModelCost) * 100}%`,
                    backgroundColor: model.color
                  }}
                ></div>
              </div>
            </div>
          ))}
        </div>

        {/* Daily Progress */}
        <div className="daily-progress">
          <div className="daily-info">
            <span>Today's Spending</span>
            <span>${MOCK_USAGE_DATA.todayUsage.toFixed(2)} / ${MOCK_USAGE_DATA.dailyBudget.toFixed(2)}</span>
          </div>
          <div className="budget-bar">
            <div className="budget-fill" style={{ width: `${(MOCK_USAGE_DATA.todayUsage / MOCK_USAGE_DATA.dailyBudget) * 100}%`, backgroundColor: MOCK_USAGE_DATA.todayUsage > MOCK_USAGE_DATA.dailyBudget * 0.8 ? 'var(--color-warning)' : 'var(--color-accent)' }}></div>
          </div>
        </div>
      </div>

      {/* 
        Mock Clipboard Toast Overlay
        This simulates what happens when a user copies text elsewhere on their PC.
      */}
      {showToast && (
        <div className="toast-overlay glass-panel">
          <div className="toast-header">
            <span className="toast-badge">{toastData.model}</span>
            <span className="toast-title">Copied to clipboard</span>
          </div>
          <div className="toast-content">
            <div className="toast-item">
              <span className="toast-val">{toastData.tokens.toLocaleString()}</span>
              <span className="toast-lbl">Tokens</span>
            </div>
            <div className="toast-item right">
              <span className="toast-val cost">${toastData.cost.toFixed(5)}</span>
              <span className="toast-lbl">Est. Cost</span>
            </div>
          </div>
        </div>
      )}

      {/* Testing Area to trigger Toast */}
      <div className="test-area" style={{ marginTop: '20px' }}>
        <div className="test-area-header">Test: Type here to simulate copying text</div>
        <textarea
          className="test-textarea glass-panel"
          placeholder="Typing here will trigger the Estimated Cost Toast popup..."
          value={inputText}
          onChange={(e) => setInputText(e.target.value)}
        />
      </div>

    </main>
  );
}

export default App;
