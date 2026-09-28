import { StateIndicator } from '@/components/custom/state-indicator';
import { PlatformType, PlatformView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateBadge } from '@/components/custom/state-badge';
import { ActivitiesTab } from '@/features/activities';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { hasCapability } from '@/lib/resource-capabilities';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { PlatformInfoActions } from '../actions';
import { usePlatformBackupSummaries } from '../platform-backups';
import { PlatformForm } from './form';
import { usePlatformGroup } from './hooks/usePlatformGroup';
import { PlatformResourceSummary, PlatformStatsTab } from './platform-stats';
import { SwarmPlatformSummary } from '@/features/swarm/platform-summary';
import { SwarmNodeAgentCoverage } from './swarm-node-agent-coverage';

type PlatformFormResource = PlatformView & RequiredFormFields;

const toFormResource = (platform: PlatformView): PlatformFormResource =>
  ({
    ...platform,
    description: platform.description ?? null,
  }) as PlatformFormResource;

const PlatformSubHeader = ({ resource }: { resource: PlatformFormResource }) => {
  const { summaries, isLoading, isError } = usePlatformBackupSummaries([resource.id]);

  return (
    <>
      {resource.type === PlatformType.DockerSwarm && (
        <div className="mt-2 flex flex-col gap-2">
          <SwarmPlatformSummary
            platformId={resource.id}
            networkCount={resource.networkCount}
            serviceStatusCounts={resource.swarmServiceStatusCounts}
            backupSummary={summaries.get(resource.id)}
            isBackupSummaryLoading={isLoading}
            isBackupSummaryError={isError}
          />
          <SwarmNodeAgentCoverage platformId={resource.id} platformName={resource.name} />
        </div>
      )}
      <PlatformResourceSummary
        platform={resource}
        backupSummary={summaries.get(resource.id)}
        isBackupSummaryLoading={isLoading}
        isBackupSummaryError={isError}
      />
    </>
  );
};

export const PlatformFormComponents: RequiredFormComponents<PlatformFormResource> = {
  AddForm: {
    Content: () => <PlatformForm mode="add" />,
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }) => (
        <StateBadge
          indicator={<StateIndicator value={resource.status} className="mr-0" kind="platform" />}
          value={resource.status}
        />
      ),
      ActionButtons: ({ resource }) => (
        <GenericActionBarButtons resource={resource} actions={[PlatformInfoActions.delete]} />
      ),
      Tags: ({ resource }) => (
        <ResourceHeaderTagsEditor
          resourceType="Platform"
          resourceId={resource.id}
          tags={resource.tags}
          disabled={!hasCapability(resource, 'canWrite')}
        />
      ),
    },
    SubHeader: PlatformSubHeader,
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => (
          <PlatformForm mode="edit" resource={resource} disabled={!hasCapability(resource, 'canWrite')} />
        ),
      },
      {
        label: 'Stats',
        Content: ({ resource }) => <PlatformStatsTab platform={resource} />,
      },
      {
        label: 'Activity',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="Platform" />,
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { platform, isLoading } = usePlatformGroup(id);
      return { item: platform ? toFormResource(platform) : undefined, isLoading };
    },
  },
};
