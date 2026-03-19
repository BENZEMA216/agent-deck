import { useState } from 'react';
import { X } from 'lucide-react';
import {
  useCreateAgent,
  type CreateAgentPayload,
} from '../model/hooks/use-create-agent';
import type { AgentType, AgentCapability } from '../model/types';

interface AgentRegisterModalProps {
  open: boolean;
  onClose: () => void;
}

type AgentTypeOption = {
  value: AgentType;
  label: string;
  configFields: { key: string; label: string; type: 'text' | 'password' }[];
  defaultCapabilities: AgentCapability[];
};

const AGENT_TYPE_OPTIONS: AgentTypeOption[] = [
  {
    value: 'openclaw',
    label: 'OpenClaw',
    configFields: [
      { key: 'url', label: 'URL', type: 'text' },
      { key: 'token', label: 'Token', type: 'password' },
    ],
    defaultCapabilities: [
      'CHAT',
      'TASK_EXECUTION',
      'HEALTH_ENDPOINT',
      'SESSION_PERSIST',
    ],
  },
  {
    value: 'claude_code',
    label: 'Claude Code',
    configFields: [
      { key: 'cli_path', label: 'CLI Path', type: 'text' },
    ],
    defaultCapabilities: ['CHAT', 'TASK_EXECUTION'],
  },
  {
    value: 'codex',
    label: 'Codex',
    configFields: [
      { key: 'cli_path', label: 'CLI Path', type: 'text' },
    ],
    defaultCapabilities: ['CHAT', 'TASK_EXECUTION'],
  },
];

export function AgentRegisterModal({ open, onClose }: AgentRegisterModalProps) {
  const [selectedType, setSelectedType] = useState<AgentType>(
    AGENT_TYPE_OPTIONS[0].value
  );
  const [displayName, setDisplayName] = useState('');
  const [configValues, setConfigValues] = useState<Record<string, string>>({});
  const createAgent = useCreateAgent();

  const typeOption =
    AGENT_TYPE_OPTIONS.find((o) => o.value === selectedType) ??
    AGENT_TYPE_OPTIONS[0];

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const payload: CreateAgentPayload = {
      agent_type: selectedType,
      display_name: displayName.trim(),
      connection_config: { ...configValues },
      capabilities: typeOption.defaultCapabilities,
    };

    createAgent.mutate(payload, {
      onSuccess: () => {
        setDisplayName('');
        setConfigValues({});
        onClose();
      },
    });
  };

  const handleTypeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setSelectedType(e.target.value);
    setConfigValues({});
  };

  const setConfigValue = (key: string, value: string) => {
    setConfigValues((prev) => ({ ...prev, [key]: value }));
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[9998] flex items-center justify-center">
      {/* Overlay */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
        onKeyDown={(e) => {
          if (e.key === 'Escape') onClose();
        }}
        role="button"
        tabIndex={-1}
        aria-label="Close modal"
      />

      {/* Modal */}
      <div className="relative z-[9999] w-full max-w-md bg-panel border border-border rounded-sm shadow-lg">
        {/* Header */}
        <div className="flex items-center justify-between p-base border-b border-border">
          <h2 className="text-lg font-medium text-high">Register Agent</h2>
          <button
            type="button"
            onClick={onClose}
            className="flex items-center justify-center w-6 h-6 rounded text-low hover:text-normal focus:outline-none focus:ring-1 focus:ring-brand"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-base space-y-4">
          {/* Agent Type */}
          <div>
            <label
              htmlFor="agent-type"
              className="block text-sm text-low mb-1"
            >
              Agent Type
            </label>
            <select
              id="agent-type"
              value={selectedType}
              onChange={handleTypeChange}
              className="w-full px-base py-1.5 bg-secondary rounded border text-base text-normal focus:outline-none focus:ring-1 focus:ring-brand"
            >
              {AGENT_TYPE_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>

          {/* Display Name */}
          <div>
            <label
              htmlFor="display-name"
              className="block text-sm text-low mb-1"
            >
              Display Name
            </label>
            <input
              id="display-name"
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="My Agent"
              required
              className="w-full px-base py-1.5 bg-secondary rounded border text-base text-normal placeholder:text-low focus:outline-none focus:ring-1 focus:ring-brand"
            />
          </div>

          {/* Dynamic config fields */}
          {typeOption.configFields.map((field) => (
            <div key={field.key}>
              <label
                htmlFor={`config-${field.key}`}
                className="block text-sm text-low mb-1"
              >
                {field.label}
              </label>
              <input
                id={`config-${field.key}`}
                type={field.type}
                value={configValues[field.key] ?? ''}
                onChange={(e) => setConfigValue(field.key, e.target.value)}
                className="w-full px-base py-1.5 bg-secondary rounded border text-base text-normal placeholder:text-low focus:outline-none focus:ring-1 focus:ring-brand"
              />
            </div>
          ))}

          {/* Error */}
          {createAgent.isError && (
            <div className="text-sm text-error">
              {createAgent.error instanceof Error
                ? createAgent.error.message
                : 'Failed to register agent.'}
            </div>
          )}

          {/* Submit */}
          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-base py-1.5 rounded border text-sm text-low hover:text-normal focus:outline-none focus:ring-1 focus:ring-brand"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={
                createAgent.isPending || displayName.trim().length === 0
              }
              className="px-base py-1.5 rounded border border-brand bg-brand/10 text-sm text-high hover:bg-brand/20 focus:outline-none focus:ring-1 focus:ring-brand disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {createAgent.isPending ? 'Registering...' : 'Register'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
