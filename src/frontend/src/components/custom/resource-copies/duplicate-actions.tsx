import { useNavigate } from 'react-router';
import { ResourceCopyKind, type ResourceCopyEligibility } from '@/api/generated/api.types';
import { DuplicateMenu } from '@/components/custom/duplicate-menu';
import { hasCapabilities } from '@/lib/resource-capabilities';

type CopySource = {
  id: string;
  platformId?: string | null;
  copyEligibility?: ResourceCopyEligibility | null;
};

export const copyDataDisabledReason = (eligibility?: ResourceCopyEligibility | null) =>
  eligibility?.status === 'Available'
    ? undefined
    : (eligibility?.reason ?? 'Copy availability could not be checked. Refresh the resource to try again.');

export function DuplicateWorkloadAction({
  resources,
  kind,
  submenu = false,
}: {
  resources: CopySource | CopySource[];
  kind: ResourceCopyKind;
  submenu?: boolean;
}) {
  const navigate = useNavigate();
  const selected = Array.isArray(resources) ? resources : [resources];
  const source = selected[0];
  const disabled = selected.length !== 1 || !hasCapabilities(source, ['canRead', 'canWrite']);
  const path = { Deployment: 'deployments', Stack: 'stacks', SwarmService: 'swarm-services' }[kind];
  return (
    <DuplicateMenu
      submenu={submenu}
      side="top"
      disabled={disabled}
      dataDisabledReason={copyDataDisabledReason(source?.copyEligibility)}
      onSelect={(mode) => {
        if (disabled || !source) return;
        if (mode === 'data') navigate(`/${path}/edit/${source.id}/?duplicate=data`);
        else {
          const addPath = kind === ResourceCopyKind.SwarmService ? `platforms/${source.platformId}/services` : path;
          navigate(`/${addPath}/add?duplicateFrom=${source.id}`);
        }
      }}
    />
  );
}
