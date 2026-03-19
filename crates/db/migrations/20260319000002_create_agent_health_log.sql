CREATE TABLE agent_health_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL REFERENCES agent_registry(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    latency_ms INTEGER,
    detail TEXT,
    checked_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_health_agent_time ON agent_health_log(agent_id, checked_at DESC);
