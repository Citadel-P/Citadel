import type { DashboardMetricSource } from '@/api/generated/api.types';
import { useFrozenDashboard } from './tile';
import { useNavigate } from 'react-router';
import { ChartNoAxesCombined, ExternalLink, FileText, Link as LinkIcon } from 'lucide-react';
import { DropdownActions } from '@/components/custom/dropdown-with-dialog';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import { WorkloadHealthSummary } from '@/features/workload-health/card';
import { workloadHealthStatus } from '@/features/workload-health/health';
import { monitoringLink } from '@/lib/monitoring-link';
export function ResourceMenu({
  kind,
  path,
  logs,
  range,
}: {
  kind: DashboardMetricSource['kind'];
  path: string;
  logs?: boolean;
  range?: number;
}) {
  const navigate = useNavigate();
  const [, copy] = useCopyToClipboard();
  return (
    <div className="shrink-0 [@media(hover:hover)]:opacity-0 group-hover:opacity-100 group-focus-within:opacity-100">
      <DropdownActions
        items={[
          { id: 'open', label: 'Open', icon: <ExternalLink />, onClick: () => navigate(path) },
          {
            id: 'stats',
            label: 'Statistics',
            icon: <ChartNoAxesCombined />,
            onClick: () => navigate(monitoringLink(path, kind, 'stats', range)),
          },
          ...(logs
            ? [
                {
                  id: 'logs',
                  label: 'Logs',
                  icon: <FileText />,
                  onClick: () => navigate(monitoringLink(path, kind, 'logs')),
                },
              ]
            : []),
          {
            id: 'copy',
            label: 'Copy link',
            icon: <LinkIcon />,
            onClick: () => void copy(new URL(path, window.location.origin).href),
          },
        ]}
      />
    </div>
  );
}
export function WorkloadDetail({ kind, id }: { kind: Exclude<DashboardMetricSource['kind'], 'Platform'>; id: string }) {
  const paused = useFrozenDashboard();
  const opts = { staleTime: 15000, retry: false };
  const deployment = useRead(
    'getDeployment',
    { deploymentId: id },
    { ...opts, enabled: !paused && kind === 'Deployment' },
  );
  const stack = useRead('getStack', { stackId: id }, { ...opts, enabled: !paused && kind === 'Stack' });
  const service = useRead('getManagedSwarmService', { id }, { ...opts, enabled: !paused && kind === 'SwarmService' });
  const query = kind === 'Stack' ? stack : kind === 'Deployment' ? deployment : service;
  if (paused) return <p>Resume the dashboard to load resource details.</p>;
  if (query.isError) return <p role="status">Resource details unavailable.</p>;
  if (!query.data) return <Loader />;
  const resource = query.data.data;
  return <WorkloadHealthSummary resource={resource} unavailable={workloadHealthStatus(resource) === 'Unknown'} />;
}
