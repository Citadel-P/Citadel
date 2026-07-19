import { BuildProjectView, BuildRunStatus } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ActivitiesTab } from '@/features/activities';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { hasCapability } from '@/lib/resource-capabilities';
import { useRead } from '@/lib/hooks';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { HubConnection } from '@microsoft/signalr';
import { useCallback, useEffect, useRef, useState } from 'react';
import { BuildInfoActions } from '../actions';
import { isActiveBuildRun, isBuildProjectActive, selectBuildProjectLatestRun } from '../build-run-state';
import { BuildForm } from './form';
import { BuildRunsTab } from './runs';

type BuildFormResource = BuildProjectView & RequiredFormFields;

export const BuildFormComponents: RequiredFormComponents<BuildFormResource> = {
  AddForm: {
    Header: {
      title: 'Build',
    },
    Content: () => <BuildForm mode="add" />,
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }) => {
        const build = resource as BuildProjectView;
        return <BuildHeaderIndicator build={build} />;
      },
      ActionButtons: ({ resource }) => {
        const { edit: _edit, ...actions } = BuildInfoActions;
        return <GenericActionBarButtons resource={resource} actions={Object.values(actions)} />;
      },
      Tags: ({ resource }) => (
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <ResourceHeaderTagsEditor
            resourceType="Build"
            resourceId={(resource as BuildProjectView).id}
            tags={(resource as BuildProjectView).tags}
            disabled={!hasCapability(resource, 'canWrite')}
          />
        </div>
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }) => (
          <BuildForm
            mode="edit"
            resource={resource as BuildProjectView}
            disabled={!hasCapability(resource, 'canWrite')}
            metadataChanged={metadataChanged}
          />
        ),
      },
      {
        label: 'Runs',
        Content: ({ resource }) => <BuildRunsTab resource={resource as BuildProjectView} />,
      },
      {
        label: 'Activities',
        Content: ({ resource }) => <ActivitiesTab resourceId={(resource as BuildProjectView).id} resourceType="Build" />,
      },
    ],
    useData(id: string) {
      const { data, isLoading } = useRead('getBuildProject', { id });
      const [build, setBuild] = useState<BuildFormResource | undefined>(data?.data as BuildFormResource | undefined);
      const lastDataRef = useRef<BuildProjectView | undefined>(data?.data);

      useEffect(() => {
        if (data?.data && data.data !== lastDataRef.current) {
          lastDataRef.current = data.data;
          setBuild((prev) => mergeBuildProjectUpdate(prev, data.data));
        }
      }, [data?.data]);

      const handleBuildProjectInfoUpdated = useCallback(
        (project: BuildProjectView) => {
          if (project.id !== id) return;

          setBuild((prev) => mergeBuildProjectUpdate(prev, project));
        },
        [id],
      );

      const setupEventListeners = useCallback(
        (hubConnection: HubConnection) => {
          hubConnection.on('BuildProjectInfoUpdated', handleBuildProjectInfoUpdated);
        },
        [handleBuildProjectInfoUpdated],
      );

      const removeEventListeners = useCallback(
        (hubConnection: HubConnection) => {
          hubConnection.off('BuildProjectInfoUpdated', handleBuildProjectInfoUpdated);
        },
        [handleBuildProjectInfoUpdated],
      );

      useSignalRGroup({
        groupName: `build-project:${id}`,
        setupEventListeners,
        removeEventListeners,
      });

      return {
        item: build,
        isLoading,
      };
    },
  },
};

function BuildHeaderIndicator({ build }: { build: BuildProjectView }) {
  const latestRun = build.latestRun;
  const isProjectProcessing = isBuildProjectActive(build);

  if (!latestRun) {
    return (
      <StateIndicator
        value={isProjectProcessing ? BuildRunStatus.Queued : build.enabled}
        isProcessing={isProjectProcessing}
        enableLabel={!isProjectProcessing}
        kind={isProjectProcessing ? 'buildRun' : undefined}
      />
    );
  }

  return (
    <StateIndicator value={latestRun.status} isProcessing={isProjectProcessing && isActiveBuildRun(latestRun)} kind="buildRun" />
  );
}

function mergeBuildProjectUpdate(prev: BuildFormResource | undefined, project: BuildProjectView): BuildFormResource {
  if (!prev) return project as BuildFormResource;

  const latestRun = selectBuildProjectLatestRun(project, prev.latestRun);
  return { ...prev, ...project, latestRun } as BuildFormResource;
}
