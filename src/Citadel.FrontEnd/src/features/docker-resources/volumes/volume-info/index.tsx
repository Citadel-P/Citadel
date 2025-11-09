import { DockerVolumeResult } from '@/api/generated/api.types';
import { Box, Info } from 'lucide-react';
import { ContainerInfoTable } from './container-info-table';
import { useRead } from '@/lib/hooks';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredDockerInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { VolumeInfoTable } from './volume-info-table';
import { VolumeInfoActions } from './actions';

export const VolumeInfoComponents: RequiredDockerInfoComponents<DockerVolumeResult> = {
  Header: {
    Indicator: ({ resource }) => {
      return <StateIndicator value={resource?.inUse ?? false} />;
    },
    ActionButtons: ({ resource }) => {
      return <GenericActionBarButtons resource={resource} actions={Object.values(VolumeInfoActions)} />;
    },
  },

  Tabs: [
    {
      label: 'Inspect',
      Content: ({ resource }) => <InspectVolumeWrapper resource={resource} />,
    },
    {
      label: 'Activity',
      Content: () => <div>Activity</div>,
    },
  ],
  useData: (platformId: string, resourceId: string) => {
    const { data, isLoading, error } = useRead(`inspectVolume`, { platformId, name: resourceId });
    return { resource: data?.data, isLoading, error };
  },
};

const InspectVolumeWrapper = ({ resource }: { resource: DockerVolumeResult }) => {
  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-col gap-2">
        <div className="flex flex-row items-center gap-2">
          <Info width={14} height={14} className="text-muted-foreground" />
          <div className="text-sm font-semibold text-muted-foreground leading-none">Details</div>
        </div>
        <div className="space-y-1 rounded-sm border p-1 shadow-xs">
          <VolumeInfoTable volume={resource} />
        </div>
      </div>
      {Object.keys(resource?.containers ?? {}).length !== 0 && (
        <div className="flex flex-col gap-2">
          <div className="flex flex-row items-center gap-2">
            <Box width={14} height={14} className="text-muted-foreground" />
            <div className="text-sm font-semibold text-muted-foreground leading-none">Containers using this volume</div>
          </div>
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <ContainerInfoTable volume={resource} />
          </div>
        </div>
      )}
    </div>
  );
};
