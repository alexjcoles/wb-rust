import { useState } from "react";

interface Props {
  onSubmit: (url: string) => void;
  loading: boolean;
}

export function BuildInput({ onSubmit, loading }: Props) {
  const [url, setUrl] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (url.trim()) onSubmit(url.trim());
  };

  return (
    <form onSubmit={handleSubmit} className="build-input">
      <input
        type="text"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="Paste a WynnBuilder URL..."
        disabled={loading}
      />
      <button type="submit" disabled={loading || !url.trim()}>
        {loading ? "Analyzing..." : "Analyze"}
      </button>
    </form>
  );
}
