import { AlertResourceType, AlertRuleStatus, type AlertRuleListItem } from '@/api/generated/api.types';

const resourceLabels: Partial<Record<AlertResourceType, string>> = {
  GitRepository: 'Git repository',
  SwarmService: 'Swarm service',
  BackupPolicy: 'Backup policy',
  BuildAgentPool: 'Build agent pool',
};
export const resourceLabel = (type: AlertResourceType) => resourceLabels[type] ?? type;

export type RuleFilters = { severities: string[]; channels: string[] };
export type RuleRow = { kind: 'rule'; id: string; rule: AlertRuleListItem };
export type RuleGroup = {
  kind: 'group';
  id: string;
  resourceType: AlertResourceType;
  rules: RuleRow[];
  enabled: number;
  total: number;
};
export type RuleTableRow = RuleRow | RuleGroup;
export const isRuleRow = (row: RuleTableRow): row is RuleRow => row.kind === 'rule';

export function groupRules(items: AlertRuleListItem[], filters: RuleFilters): RuleGroup[] {
  const groups = new Map<AlertResourceType, RuleGroup>();
  for (const rule of items) {
    let group = groups.get(rule.resourceType);
    if (!group) {
      group = {
        kind: 'group',
        id: `group:${rule.resourceType}`,
        resourceType: rule.resourceType,
        rules: [],
        enabled: 0,
        total: 0,
      };
      groups.set(rule.resourceType, group);
    }
    group.total++;
    if (rule.status === AlertRuleStatus.Enabled) group.enabled++;
    if (filters.severities.length && !filters.severities.includes(rule.severity)) continue;
    if (filters.channels.length && !rule.channelIds.some((id) => filters.channels.includes(id))) continue;
    group.rules.push({ kind: 'rule', id: rule.id, rule });
  }
  return [...groups.values()]
    .filter((group) => group.rules.length)
    .sort((a, b) => resourceLabel(a.resourceType).localeCompare(resourceLabel(b.resourceType)));
}
