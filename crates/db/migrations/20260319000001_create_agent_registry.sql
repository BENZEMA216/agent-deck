CREATE TABLE agent_registry (
    id TEXT PRIMARY KEY NOT NULL,
    agent_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    connection_config TEXT NOT NULL,
    capabilities TEXT NOT NULL DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
