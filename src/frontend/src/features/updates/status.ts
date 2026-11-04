import type { AgentTargetView } from '@/api/generated/api.types';

export const agentStatusLabels: Record<AgentTargetView['status'], string> = {
  Available: 'Update available',
  Current: 'Up to date',
  Offline: 'Offline',
  Unknown: 'Version unknown',
  ManualRequired: 'Manual update required',
  Incompatible: 'No compatible update',
};
