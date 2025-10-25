import { DockerNetworkDetails, DockerNetworkResult } from '@/api/generated/api.types';
import { Box, Info, Share2 } from 'lucide-react';
import { ContainerInfoTable } from './container-info-table';
import { NetworkInfoTable } from './network-info-table';
import { IPAMInfoTable } from './ipam-info-table';
import { useRead } from '@/lib/hooks';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { DeleteNetworkButton } from '../actions';
import { GenericActionBarButtons } from '@/components/custom/action-bar';

export const NetworkInfoComponents: RequiredDockerInfoComponents<DockerNetworkDetails> = {
  Header: {
    Indicator: ({ resource }) => {
      return <StateIndicator value={Object.keys(resource.containers ?? {}).length > 0} />;
    },
    ActionButtons: ({ resource }) => {
      return (
        <GenericActionBarButtons
          resource={
            {
              id: resource.id,
              name: resource.name,
              inUse: Object.keys(resource.containers ?? {}).length > 0,
            } as DockerNetworkResult
          }
          actions={[DeleteNetworkButton]}
        />
      );
    },
  },

  Tabs: [
    {
      label: 'Inspect',
      Content: ({ resource }) => <InspectNetworkWrapper resource={resource} />,
    },
    {
      label: 'Activity',
      Content: () => <div>Activity</div>,
    },
  ],
  useData: (platformId: string, resourceId: string) => {
    const { data, isLoading, error } = useRead(`inspectNetwork`, { platformId, networkId: resourceId });
    return { resource: data?.data, isLoading, error };
  },
};

const InspectNetworkWrapper = ({ resource }: { resource: DockerNetworkDetails }) => {
  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-col gap-2">
        <div className="flex flex-row items-center gap-2">
          <Info width={14} height={14} className="text-muted-foreground" />
          <div className="text-sm font-semibold text-muted-foreground leading-none">Details</div>
        </div>
        <div className="space-y-1 rounded-sm border p-1 shadow-xs">
          <NetworkInfoTable network={resource} />
        </div>
      </div>
      {Object.keys(resource?.containers ?? {}).length !== 0 && (
        <div className="flex flex-col gap-2">
          <div className="flex flex-row items-center gap-2">
            <Box width={14} height={14} className="text-muted-foreground" />
            <div className="text-sm font-semibold text-muted-foreground leading-none">Containers in this network</div>
          </div>
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <ContainerInfoTable network={resource} />
          </div>
        </div>
      )}

      {resource?.ipam?.config?.length !== 0 && (
        <div className="flex flex-col gap-2">
          <div className="flex flex-row items-center gap-2">
            <Share2 width={14} height={14} className="text-muted-foreground" />
            <div className="text-sm font-semibold text-muted-foreground leading-none">IPAM</div>
          </div>
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <IPAMInfoTable ipam={resource?.ipam ?? undefined} />
          </div>
        </div>
      )}
    </div>
  );
};
