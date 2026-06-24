import { useEffect, useState } from "react";
import { historyGet, historyClear } from "../shared/tauri-api";
import type { Conversation } from "../shared/types";
import "../styles/history.css";

const LANG_LABEL: Record<string, string> = {
  vi: "Tiếng Việt",
  ja: "日本語",
  en: "English",
};

function formatTime(ms: number): string {
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return "";
  }
}

/** History window: conversations newest-first, each shown as a thread. */
export function HistoryView() {
  const [conversations, setConversations] = useState<Conversation[] | null>(null);

  const load = () => {
    void historyGet()
      .then(setConversations)
      .catch(() => setConversations([]));
  };
  useEffect(load, []);

  const onClear = async () => {
    if (!window.confirm("Clear all translation history?")) return;
    await historyClear();
    setConversations([]);
  };

  return (
    <main className="history">
      <header className="history-head">
        <h1>History</h1>
        <button
          className="clear-btn"
          onClick={() => void onClear()}
          disabled={!conversations || conversations.length === 0}
        >
          Clear history
        </button>
      </header>

      {conversations === null ? (
        <p className="history-empty">Loading…</p>
      ) : conversations.length === 0 ? (
        <p className="history-empty">No translations yet.</p>
      ) : (
        <ul className="conv-list">
          {conversations.map((c) => (
            <li key={c.id} className="conv">
              <div className="conv-meta">
                <span className="conv-lang">{LANG_LABEL[c.target_lang] ?? c.target_lang}</span>
                <time>{formatTime(c.created_at)}</time>
              </div>
              {c.turns.map((t, i) => (
                <div key={i} className={`turn turn-${t.kind}`}>
                  <p className="turn-out">{t.output}</p>
                  <p className="turn-in">{t.input}</p>
                </div>
              ))}
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
