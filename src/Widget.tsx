import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import "./App.css";

type Usage = {
  totalBilled: number;
  todayUsage: number;
};

export default function Widget() {
  const [usage, setUsage] = useState<Usage | null>(null);

  useEffect(() => {
    let interval: number;
    
    const fetchUsage = async () => {
      try {
        if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
          const store = await Store.load("settings.json");
          const key = (await store.get<string>("tokenglass_openai_admin_key")) ?? "";
          if (key) {
            const result = await invoke<Usage>("fetch_openai_usage", { adminKey: key });
            setUsage(result);
          }
        }
      } catch (error) {
        console.error("위젯 사용량 조회 오류:", error);
      }
    };

    void fetchUsage();
    // 5분 단위 갱신
    interval = window.setInterval(() => void fetchUsage(), 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="widget-root" data-tauri-drag-region>
      <div className="widget-content" data-tauri-drag-region>
        <span className="widget-label" data-tauri-drag-region>Today</span>
        <span className="widget-value" data-tauri-drag-region>{usage ? `$${usage.todayUsage.toFixed(2)}` : "—"}</span>
      </div>
    </div>
  );
}
