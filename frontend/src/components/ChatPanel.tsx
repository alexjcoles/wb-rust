import { useState } from "react";
import { chat } from "../api/client";
import type { ChatResponse } from "../types/build";

interface Props {
  buildUrl: string;
}

interface Message {
  role: "user" | "assistant";
  content: string;
}

export function ChatPanel({ buildUrl }: Props) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);

  const sendMessage = async (text: string) => {
    const userMsg: Message = { role: "user", content: text };
    const history = [...messages, userMsg];
    setMessages(history);
    setInput("");
    setLoading(true);

    try {
      // Include the build URL in context if this is the first message
      const contextMsg =
        history.length === 1
          ? `${text}\n\nBuild URL: ${buildUrl}`
          : text;

      const apiMessages = history.map((m, i) => ({
        role: m.role,
        content: i === history.length - 1 ? contextMsg : m.content,
      }));

      const response: ChatResponse = await chat(apiMessages);
      setMessages([...history, { role: "assistant", content: response.text }]);
    } catch (err) {
      setMessages([
        ...history,
        { role: "assistant", content: `Error: ${err}` },
      ]);
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (input.trim() && !loading) sendMessage(input.trim());
  };

  const handleQuickAction = (prompt: string) => {
    if (!loading) sendMessage(prompt);
  };

  return (
    <div className="chat-panel">
      <div className="chat-header">
        <h3>AI Build Advisor</h3>
        {messages.length === 0 && (
          <div className="quick-actions">
            <button onClick={() => handleQuickAction("Analyze this build and suggest improvements")}>
              Analyze & Improve
            </button>
            <button onClick={() => handleQuickAction("How can I improve survivability?")}>
              Improve Survivability
            </button>
            <button onClick={() => handleQuickAction("Fix my elemental defences")}>
              Fix Defences
            </button>
          </div>
        )}
      </div>

      <div className="chat-messages">
        {messages.map((msg, i) => (
          <div key={i} className={`chat-msg chat-msg-${msg.role}`}>
            <div className="chat-msg-content" dangerouslySetInnerHTML={{
              __html: msg.role === "assistant" ? simpleMarkdown(msg.content) : escapeHtml(msg.content)
            }} />
          </div>
        ))}
        {loading && (
          <div className="chat-msg chat-msg-assistant">
            <div className="chat-msg-content loading">Thinking...</div>
          </div>
        )}
      </div>

      <form onSubmit={handleSubmit} className="chat-input">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Ask about your build..."
          disabled={loading}
        />
        <button type="submit" disabled={loading || !input.trim()}>
          Send
        </button>
      </form>
    </div>
  );
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function simpleMarkdown(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    // Headers
    .replace(/^### (.+)$/gm, "<h4>$1</h4>")
    .replace(/^## (.+)$/gm, "<h3>$1</h3>")
    // Bold
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    // Links
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
    // Tables
    .replace(/^\|(.+)\|$/gm, (match) => {
      const cells = match.split("|").filter(Boolean).map((c) => c.trim());
      if (cells.every((c) => /^-+$/.test(c))) return "";
      const tag = "td";
      return "<tr>" + cells.map((c) => `<${tag}>${c}</${tag}>`).join("") + "</tr>";
    })
    // List items
    .replace(/^(\d+)\. (.+)$/gm, "<li>$2</li>")
    .replace(/^- (.+)$/gm, "<li>$1</li>")
    // Line breaks
    .replace(/\n\n/g, "<br><br>")
    .replace(/\n/g, "<br>");
}
