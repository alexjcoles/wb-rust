import { useState } from "react";
import { BuildInput } from "./components/BuildInput";
import { BuildDisplay } from "./components/BuildDisplay";
import { ChatPanel } from "./components/ChatPanel";
import { analyzeBuild, parseBuild } from "./api/client";
import type { AnalyzeResponse, ParseResponse } from "./types/build";
import "./App.css";

function App() {
  const [buildUrl, setBuildUrl] = useState("");
  const [parseData, setParseData] = useState<ParseResponse | null>(null);
  const [analyzeData, setAnalyzeData] = useState<AnalyzeResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (url: string) => {
    setLoading(true);
    setError(null);
    setBuildUrl(url);

    try {
      const [parsed, analyzed] = await Promise.all([
        parseBuild(url),
        analyzeBuild(url),
      ]);
      setParseData(parsed);
      setAnalyzeData(analyzed);
    } catch (err) {
      setError(String(err));
      setParseData(null);
      setAnalyzeData(null);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>Wynn Build Advisor</h1>
        <p>Paste a WynnBuilder URL to analyze your build</p>
      </header>

      <BuildInput onSubmit={handleSubmit} loading={loading} />

      {error && <div className="error">{error}</div>}

      {analyzeData && parseData && (
        <div className="main-content">
          <div className="left-panel">
            <div className="items-list">
              <h3>Equipment</h3>
              {parseData.items.map((item) => (
                <div key={item.slot} className="item-row">
                  <span className="item-slot">{item.slot}</span>
                  <span className="item-name">{item.name}</span>
                </div>
              ))}
            </div>
            <BuildDisplay data={analyzeData} />
          </div>

          <div className="right-panel">
            <ChatPanel buildUrl={buildUrl} />
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
