import { CitadelIcons } from '@/lib/icons';
import { ChartNoAxesCombined, CalendarClock, NotebookText } from 'lucide-react';
import {
  DashboardChartColor,
  DashboardChartStyle,
  DashboardMetric,
  DashboardWorkloadKind,
  type DashboardConfiguration,
  type DashboardWidget,
  type DashboardWidgetSettings,
} from '@/api/generated/api.types';
import { STATS_PRESETS } from '@/lib/stats-range';
import { randomUuid } from '@/lib/utils';

export type WidgetKind = DashboardWidgetSettings['kind'];
export const categories = ['Overview', 'Infrastructure', 'Monitoring', 'Operations', 'Content'] as const;
function definition(
  label: string,
  description: string,
  Icon: typeof CitadelIcons.Platform,
  category: (typeof categories)[number],
  w: number,
  h: number,
  minW = 2,
  minH = 2,
) {
  return { label, description, Icon, category, size: { w, h }, minW, minH };
}
export const catalogue = {
  ResourceSummary: definition(
    'Resource summary',
    'Counts for visible resources and open alerts.',
    CitadelIcons.Dashboard,
    'Overview',
    12,
    2,
  ),
  Platforms: definition(
    'Platforms',
    'Connectivity, CPU and memory by platform.',
    CitadelIcons.Platform,
    'Infrastructure',
    8,
    3,
  ),
  Workloads: definition(
    'Workloads',
    'Runtime status, container probes and backup coverage.',
    CitadelIcons.Deployment,
    'Infrastructure',
    8,
    4,
  ),
  Metrics: definition(
    'Metrics',
    'Chart measurements for one resource over time.',
    ChartNoAxesCombined,
    'Monitoring',
    8,
    5,
    4,
    3,
  ),
  OpenAlerts: definition(
    'Open alerts',
    'Unresolved alerts and their severity.',
    CitadelIcons.Alert,
    'Monitoring',
    4,
    3,
  ),
  RecentActivity: definition(
    'Recent activity',
    'Recent operations you can read.',
    CitadelIcons.Activity,
    'Operations',
    6,
    4,
  ),
  Backups: definition(
    'Backup activity',
    'Results of recently completed backups.',
    CitadelIcons.BackupRepository,
    'Operations',
    6,
    4,
  ),
  UpcomingSchedules: definition(
    'Upcoming schedules',
    'The next scheduled occurrences.',
    CalendarClock,
    'Operations',
    6,
    3,
  ),
  Updates: definition('Updates', 'Release information for this instance.', CitadelIcons.Updates, 'Operations', 4, 3),
  Notes: definition('Notes and links', 'Text and links for this dashboard.', NotebookText, 'Content', 4, 3),
} satisfies Record<WidgetKind, ReturnType<typeof definition>>;

export function metricSettings(metric = DashboardMetric.Cpu) {
  return {
    kind: 'Metrics' as const,
    source: { kind: 'Platform' as const, id: '' },
    metric,
    presentation: DashboardChartStyle.Gradient,
    color: DashboardChartColor.Blue,
    legend: true,
  };
}

export function makeWidget(kind: WidgetKind): DashboardWidget {
  let settings: DashboardWidgetSettings;
  switch (kind) {
    case 'ResourceSummary':
    case 'Updates':
      settings = { kind };
      break;
    case 'Notes':
      settings = { kind, text: '', links: [] };
      break;
    case 'Metrics':
      settings = metricSettings();
      break;
    case 'Workloads':
      settings = { kind, resourceKind: DashboardWorkloadKind.Deployment, limit: 5 };
      break;
    default:
      settings = { kind, limit: 5 };
  }
  return { id: randomUuid(), title: catalogue[kind].label, settings, layout: { x: 0, y: 0, ...catalogue[kind].size } };
}

export function validLink(href: string) {
  if (
    !href ||
    href.trim() !== href ||
    href.includes('\\') ||
    [...href].some((c) => c.charCodeAt(0) < 32 || c.charCodeAt(0) === 127) ||
    href.length > 2048
  )
    return false;
  if (href.startsWith('/')) return !href.startsWith('//');
  try {
    const u = new URL(href);
    return ['https:', 'http:'].includes(u.protocol) && !u.username && !u.password;
  } catch {
    return false;
  }
}

export function widgetSourceError(s: DashboardWidgetSettings) {
  if ('limit' in s && (!Number.isInteger(s.limit) || s.limit < 1 || s.limit > 10))
    return 'Choose between one and ten rows.';
  if (s.kind === 'Metrics' && (!s.source.id || (s.source.kind === 'Container' && !s.source.platformId)))
    return 'Choose a metric resource.';
}

export function widgetError(widget: DashboardWidget) {
  const { settings: s } = widget;
  if (!widget.title.trim() || [...widget.title].length > 128) return 'Enter a widget title of up to 128 characters.';
  if (
    s.kind === 'Notes' &&
    ([...s.text].length > 2048 ||
      s.links.length > 8 ||
      s.links.some((l) => !l.label.trim() || [...l.label].length > 128 || !validLink(l.href)))
  )
    return 'Check the note length and enter a label and valid address for each link.';
  return widgetSourceError(s);
}

const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const integer = (v: unknown, min: number, max: number): v is number =>
  typeof v === 'number' && Number.isInteger(v) && v >= min && v <= max;
function supportedSettings(s: unknown): s is DashboardWidgetSettings {
  if (!record(s) || typeof s.kind !== 'string' || !Object.hasOwn(catalogue, s.kind)) return false;
  switch (s.kind) {
    case 'Notes':
      return (
        typeof s.text === 'string' &&
        Array.isArray(s.links) &&
        s.links.every((l) => record(l) && typeof l.label === 'string' && typeof l.href === 'string')
      );
    case 'Metrics':
      return (
        record(s.source) &&
        typeof s.source.id === 'string' &&
        ['Platform', ...Object.values(DashboardWorkloadKind)].includes(String(s.source.kind)) &&
        (s.source.kind !== 'Container' || typeof s.source.platformId === 'string') &&
        Object.hasOwn(DashboardMetric, String(s.metric)) &&
        Object.hasOwn(DashboardChartColor, String(s.color)) &&
        Object.hasOwn(DashboardChartStyle, String(s.presentation)) &&
        typeof s.legend === 'boolean'
      );
    case 'ResourceSummary':
    case 'Updates':
      return true;
    case 'Workloads':
      return Object.hasOwn(DashboardWorkloadKind, String(s.resourceKind)) && integer(s.limit, 1, 10);
    default:
      return integer(s.limit, 1, 10);
  }
}
export function supportedConfiguration(c: unknown): c is DashboardConfiguration {
  return (
    record(c) &&
    c.schemaVersion === 1 &&
    STATS_PRESETS.some((p) => p.minutes === c.rangeMinutes) &&
    Array.isArray(c.tags) &&
    c.tags.every((t) => typeof t === 'string') &&
    Array.isArray(c.widgets) &&
    c.widgets.length <= 24 &&
    c.widgets.every(
      (w) =>
        record(w) &&
        typeof w.id === 'string' &&
        typeof w.title === 'string' &&
        supportedSettings(w.settings) &&
        record(w.layout) &&
        integer(w.layout.x, 0, 10) &&
        integer(w.layout.w, 2, 12 - w.layout.x) &&
        integer(w.layout.y, 0, 512) &&
        integer(w.layout.h, 2, 24),
    )
  );
}
