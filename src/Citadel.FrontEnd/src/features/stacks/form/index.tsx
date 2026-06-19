import { RequiredFormComponents, RequiredFormFields } from '@/pages/types';
import { StackForm } from './form';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StackActions } from './actions';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useStackGroup } from './hooks/useStackGroup';
import { StackView, ResourceControlState, LatestActivityView, ActivityStatus } from '@/api/generated/api.types';
import { ActivitiesTab } from '@/features/activities';
import { hasCapability } from '@/lib/resource-capabilities';
import { AlertMessage } from '@/components/custom/alert-message';
import { ActivityAlertZone } from '@/components/custom/task-sheet';

export const StackFormComponents: RequiredFormComponents = {
  AddForm: {
    Content: () => {
      return <StackForm mode="add" />;
    },
  },
  EditForm: {
    Header: {
      Indicator: ({ resource }: { resource: RequiredFormFields }) => {
        return (
          <StateIndicator
            value={(resource as StackView).status}
            isProcessing={(resource as StackView).controlState === ResourceControlState.Processing}
          />
        );
      },
      ActionButtons: ({ resource }) => {
        return <GenericActionBarButtons resource={resource} actions={Object.values(StackActions)} />;
      },
    },
    SubHeader: ({ resource }: { resource: StackView }) => {
      return <StackSubHeader latestActivity={resource.latestActivityView ?? null} stack={resource} />;
    },
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource, metadataChanged }: { resource: StackView; metadataChanged?: boolean }) => {
          return (
            <StackForm mode="edit" metadataChanged={metadataChanged} disabled={!hasCapability(resource, 'canWrite')} />
          );
        },
      },
      {
        label: 'Activities',
        Content: ({ resource }: { resource: StackView }) => {
          return <ActivitiesTab resourceId={resource.id} resourceType="Stack" />;
        },
      },
    ],
    useData: function (id: string): { item?: RequiredFormFields; isLoading: boolean } {
      const { stack, isLoading } = useStackGroup(id);
      return { item: stack, isLoading };
    },
  },
};

const StackSubHeader = ({ stack, latestActivity }: { stack: StackView; latestActivity: LatestActivityView | null }) => {
  return (
    <>
      <StackLatestActivity latestActivity={latestActivity} />
      {/* <StackUpdateNotice stack={stack} /> */}
    </>
  );
};
const StackLatestActivity = ({ latestActivity }: { latestActivity: LatestActivityView | null }) => {
  if (!latestActivity) return;
  if (latestActivity?.status === ActivityStatus.Success) {
    return;
  }
  if (latestActivity?.info.$type === 'StackDegraded') {
    return (
      <AlertMessage date={latestActivity?.createdAt} type={'warning'}>
        <div className="flex flex-wrap gap-2 items-center ">{latestActivity?.info.reason}</div>
      </AlertMessage>
    );
  }
  return (
    <ActivityAlertZone
      info={latestActivity?.info as any}
      activity={latestActivity as any}
      title="Error"
      date={latestActivity?.createdAt}
    />
  );
};