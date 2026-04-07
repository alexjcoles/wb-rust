import type { AnalyzeResponse } from "../types/build";

interface Props {
  data: AnalyzeResponse;
}

const DEFENCE_ELEMENTS = [
  { key: "earth_defence" as const, label: "Earth", color: "#4a7" },
  { key: "thunder_defence" as const, label: "Thunder", color: "#dd4" },
  { key: "water_defence" as const, label: "Water", color: "#4ad" },
  { key: "fire_defence" as const, label: "Fire", color: "#d44" },
  { key: "air_defence" as const, label: "Air", color: "#aaa" },
];

export function BuildDisplay({ data }: Props) {
  const { stats, archetype, survivability_score, dps_score, issues } = data;

  return (
    <div className="build-display">
      <div className="scores-row">
        <div className="score-card">
          <span className="score-label">Archetype</span>
          <span className="score-value">{archetype}</span>
        </div>
        <div className="score-card">
          <span className="score-label">Survivability</span>
          <span className="score-value">{survivability_score.toFixed(0)}</span>
        </div>
        <div className="score-card">
          <span className="score-label">DPS</span>
          <span className="score-value">{dps_score.toFixed(0)}</span>
        </div>
      </div>

      <div className="stats-grid">
        <div className="stat-section">
          <h3>Combat</h3>
          <StatRow label="HP" value={stats.hp} />
          <StatRow label="EHP" value={Math.round(stats.ehp)} />
          <StatRow label="HPR" value={stats.hpr} />
          <StatRow label="Life Steal" value={stats.life_steal} />
          <StatRow label="Mana Regen" value={stats.mana_regen} />
          <StatRow label="Walk Speed" value={stats.walk_speed} suffix="%" />
          <StatRow label="SP Used" value={stats.assigned_sp_total} warn={stats.assigned_sp_total > 200} />
        </div>

        <div className="stat-section">
          <h3>Offence</h3>
          <StatRow label="Spell Damage Raw" value={stats.spell_damage_raw} />
          <StatRow label="Spell Damage %" value={stats.spell_damage_pct} suffix="%" />
        </div>

        <div className="stat-section">
          <h3>Defences</h3>
          {DEFENCE_ELEMENTS.map((el) => (
            <StatRow
              key={el.key}
              label={el.label}
              value={stats[el.key]}
              color={el.color}
              warn={stats[el.key] < -60}
              bad={stats[el.key] < 0}
            />
          ))}
        </div>
      </div>

      {issues.length > 0 && (
        <div className="issues">
          <h3>Issues</h3>
          {issues.map((issue, i) => (
            <div key={i} className={`issue issue-${issue.severity}`}>
              <span className="issue-badge">{issue.severity}</span>
              {issue.description}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function StatRow({
  label,
  value,
  suffix = "",
  color,
  warn,
  bad,
}: {
  label: string;
  value: number;
  suffix?: string;
  color?: string;
  warn?: boolean;
  bad?: boolean;
}) {
  const className = warn ? "stat-row stat-warn" : bad ? "stat-row stat-bad" : "stat-row";
  return (
    <div className={className}>
      <span className="stat-label" style={color ? { color } : undefined}>
        {label}
      </span>
      <span className="stat-value">
        {value.toLocaleString()}
        {suffix}
      </span>
    </div>
  );
}
