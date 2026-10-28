import { StateBadge } from '@/components/custom/state-badge';
import { AuthorizedScript, ResourceControlState } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ActivitiesTab } from '@/features/activities';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { useScriptGroup } from '../hooks/useScriptGroup';
import { hasCapability } from '@/lib/resource-capabilities';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { ScriptInfoActions } from '../actions';
import { ScriptForm } from './form';
import { ScriptRunsTab } from './runs';

type ScriptFormResource = AuthorizedScript & RequiredFormFields;

export const ScriptFormComponents: RequiredFormComponents<ScriptFormResource> = {
  AddForm: {
    Header: {
      title: 'Script',
    },
    Content: () => <ScriptForm mode="add" />,
  },
  EditForm: {
    Header: {
      canEditTitle: true,
      canEditDescription: true,
      Indicator: ({ resource }) => {
        const script = resource as AuthorizedScript;
        const status = script.latestRun?.status ?? script.enabled;
        return (
          <StateBadge
            value={status}
            kind={typeof status === 'boolean' ? 'default' : 'run'}
            label={typeof status === 'boolean' ? (status ? 'Enabled' : 'Disabled') : undefined}
            isProcessing={script.controlState === ResourceControlState.Processing}
            indicator={
              <StateIndicator
                value={status}
                enableLabel={typeof status === 'boolean'}
                kind={typeof status === 'boolean' ? undefined : 'scriptRun'}
                className="mr-0"
              />
            }
          />
        );
      },
      ActionButtons: ({ resource }) => {
        const { edit: _edit, duplicate: _duplicate, ...actions } = ScriptInfoActions;
        return <GenericActionBarButtons resource={resource} actions={Object.values(actions)} />;
      },
      Tags: ({ resource }) => (
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <ResourceHeaderTagsEditor
            resourceType="Script"
            resourceId={resource.id}
            tags={(resource as AuthorizedScript).tags}
            disabled={!hasCapability(resource, 'canWrite')}
          />
          <ScriptInfoActions.duplicate resource={resource} />
        </div>
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }) => (
          <ScriptForm
            mode="edit"
            resource={resource as ScriptFormResource}
            metadataChanged={metadataChanged}
            disabled={!hasCapability(resource, 'canWrite')}
          />
        ),
      },
      {
        label: 'Runs',
        Content: ({ resource }) => <ScriptRunsTab resource={resource as AuthorizedScript} />,
      },
      {
        label: 'Activities',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="Script" />,
      },
    ],
    useData(id: string) {
      const { data, isLoading, error, refetch, isFetching } = useScriptGroup(id);
      return {
        error,
        refetch,
        isFetching,
        item: data?.data
          ? ({
              ...data.data,
              status: data.data.latestRun?.status ?? data.data.enabled,
              description: data.data.description,
            } satisfies ScriptFormResource)
          : undefined,
        isLoading,
      };
    },
  },
};
