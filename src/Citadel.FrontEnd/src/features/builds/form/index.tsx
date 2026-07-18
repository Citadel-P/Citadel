import { BuildProjectView } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { hasCapability } from '@/lib/resource-capabilities';
import { useRead } from '@/lib/hooks';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { HubConnection } from '@microsoft/signalr';
import { useCallback, useEffect, useRef, useState } from 'react';
import { BuildInfoActions } from '../actions';
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
        return <StateIndicator value={build.enabled} isProcessing={false} enableLabel />;
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
    ],
    useData(id: string) {
      const { data, isLoading } = useRead('getBuildProject', { id });
      const [build, setBuild] = useState<BuildFormResource | undefined>(data?.data as BuildFormResource | undefined);
      const lastDataRef = useRef<BuildProjectView | undefined>(data?.data);

      useEffect(() => {
        if (data?.data && data.data !== lastDataRef.current) {
          lastDataRef.current = data.data;
          setBuild(data.data as BuildFormResource);
        }
      }, [data?.data]);

      const handleBuildProjectInfoUpdated = useCallback((project: BuildProjectView) => {
        setBuild((prev) => {
          if (!prev) return project as BuildFormResource;
          return { ...prev, ...project };
        });
      }, []);

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
