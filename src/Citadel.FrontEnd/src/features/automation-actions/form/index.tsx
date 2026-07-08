import { AutomationActionView, ResourceControlState } from '@/api/generated/api.types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ActivitiesTab } from '@/features/activities';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { useRead } from '@/lib/hooks';
import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { AutomationActionInfoActions } from '../actions';
import { AutomationActionForm } from './form';
import { AutomationActionRunsTab } from './runs';

type AutomationActionFormResource = AutomationActionView & RequiredFormFields;

export const AutomationActionFormComponents: RequiredFormComponents<AutomationActionFormResource> = {
  AddForm: {
    Header: {
      title: 'Automation Action',
    },
    Content: () => <AutomationActionForm mode="add" />,
  },
  EditForm: {
    Header: {
      canEditTitle: true,
      canEditDescription: true,
      Indicator: ({ resource }) => {
        const action = resource as AutomationActionView;
        const status = action.latestRun?.status ?? action.enabled;
        return (
          <StateIndicator
            value={status}
            isProcessing={action.controlState === ResourceControlState.Processing}
            enableLabel={typeof status === 'boolean'}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        const { edit: _edit, ...actions } = AutomationActionInfoActions;
        return <GenericActionBarButtons resource={resource} actions={Object.values(actions)} />;
      },
      Tags: ({ resource }) => (
        <ResourceHeaderTagsEditor
          resourceType="AutomationAction"
          resourceId={resource.id}
          tags={(resource as AutomationActionView).tags}
        />
      ),
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }) => (
          <AutomationActionForm
            mode="edit"
            resource={resource as AutomationActionFormResource}
            metadataChanged={metadataChanged}
          />
        ),
      },
      {
        label: 'Runs',
        Content: ({ resource }) => <AutomationActionRunsTab resource={resource as AutomationActionView} />,
      },
      {
        label: 'Activities',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="AutomationAction" />,
      },
    ],
    useData(id: string) {
      const { data, isLoading } = useRead('getAutomationAction', { id });
      return {
        item: data?.data
          ? ({
              ...data.data,
              status: data.data.latestRun?.status ?? data.data.enabled,
              description: data.data.description,
              capabilities: { canRead: true, canWrite: true, canExecute: true },
            } satisfies AutomationActionFormResource)
          : undefined,
        isLoading,
      };
    },
  },
};
