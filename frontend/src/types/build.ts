export interface SlotItem {
  slot: string;
  name: string;
  id: number;
}

export interface ParseResponse {
  items: SlotItem[];
  level: number;
  assigned_sp: SpValues | null;
}

export interface SpValues {
  earth: number;
  thunder: number;
  water: number;
  fire: number;
  air: number;
}

export interface AnalyzeResponse {
  archetype: string;
  survivability_score: number;
  dps_score: number;
  stats: StatsSnapshot;
  issues: Issue[];
}

export interface StatsSnapshot {
  hp: number;
  ehp: number;
  hpr: number;
  life_steal: number;
  mana_regen: number;
  spell_damage_raw: number;
  spell_damage_pct: number;
  walk_speed: number;
  earth_defence: number;
  thunder_defence: number;
  water_defence: number;
  fire_defence: number;
  air_defence: number;
  assigned_sp_total: number;
}

export interface Issue {
  severity: "critical" | "warning" | "info";
  category: string;
  description: string;
}

export interface ChatResponse {
  text: string;
  provider: string;
  tool_calls: { name: string; arguments: unknown }[];
}
