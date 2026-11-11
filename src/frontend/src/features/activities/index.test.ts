import { ActivityEventType, ActivityResourceType } from '@/api/generated/api.types';
import { getActivityEventOptions, getActivityResourceOptions } from './index';

describe('activity resource options', () => {
  it('offers script lifecycle, run and webhook events when filtering by Script', () => {
    const options = getActivityEventOptions(ActivityResourceType.Script);

    expect(options).toEqual(
      expect.arrayContaining([
        ActivityEventType.ScriptCreated,
        ActivityEventType.ScriptUpdated,
        ActivityEventType.ScriptRenamed,
        ActivityEventType.ScriptDeleted,
        ActivityEventType.ScriptRunQueued,
        ActivityEventType.ScriptRunStarted,
        ActivityEventType.ScriptRunSucceeded,
        ActivityEventType.ScriptRunFailed,
        ActivityEventType.ScriptRunTimedOut,
        ActivityEventType.ScriptRunCancelled,
        ActivityEventType.ScriptRunRejected,
        ActivityEventType.ScriptRunCleanupResolved,
        ActivityEventType.ScriptWebhookReceived,
      ]),
    );
    expect(options).not.toContain(ActivityEventType.DeploymentCreated);
    expect(options).not.toContain(ActivityEventType.BuildRunFailed);
  });

  it('offers failed backup runs globally and for backup policies, but not other resources', () => {
    const option = { value: 'BackupRunFailed', label: 'Backup run failed' };
    expect(getActivityEventOptions('All')).toContainEqual(option);
    expect(getActivityEventOptions(ActivityResourceType.BackupPolicy)).toContainEqual(option);
    expect(getActivityEventOptions(ActivityResourceType.Deployment)).not.toContainEqual(option);
  });
  it('hides Service Accounts from non-administrators', () => {
    const options = getActivityResourceOptions(false);

    expect(options).not.toContainEqual(expect.objectContaining({ value: ActivityResourceType.ServiceAccount }));
  });

  it('shows Service Accounts to administrators', () => {
    const options = getActivityResourceOptions(true);

    expect(options).toContainEqual(
      expect.objectContaining({ value: ActivityResourceType.ServiceAccount, label: 'Service Account' }),
    );
  });
});

it('keeps Updates administrator-only and includes all singleton update event types', () => {
  expect(getActivityResourceOptions(false)).not.toContainEqual(
    expect.objectContaining({ value: ActivityResourceType.Updates }),
  );
  expect(getActivityResourceOptions(true)).toContainEqual(
    expect.objectContaining({ value: ActivityResourceType.Updates }),
  );
  expect(getActivityEventOptions(ActivityResourceType.Updates)).toEqual(
    expect.arrayContaining([
      ActivityEventType.AgentUpdate,
      ActivityEventType.AgentUpdateScheduleChanged,
      ActivityEventType.UpdateDiscovered,
      ActivityEventType.UpdateChecksChanged,
      ActivityEventType.InstanceUpdateStarted,
      ActivityEventType.InstanceUpdateCompleted,
      ActivityEventType.InstanceUpdateFailed,
      ActivityEventType.InstanceUpdatePostponed,
      ActivityEventType.InstanceUpdateRecovered,
    ]),
  );
});

it('keeps Instance administrator-only and separate from Updates', () => {
  expect(getActivityResourceOptions(false)).not.toContainEqual(
    expect.objectContaining({ value: ActivityResourceType.Instance }),
  );
  expect(getActivityResourceOptions(true)).toContainEqual(
    expect.objectContaining({ value: ActivityResourceType.Instance }),
  );
  expect(getActivityEventOptions(ActivityResourceType.Instance)).toEqual([
    ActivityEventType.InstanceConfigurationUpdated,
    ActivityEventType.InstanceRestarted,
  ]);
  expect(getActivityEventOptions(ActivityResourceType.Updates)).not.toContain(
    ActivityEventType.InstanceConfigurationUpdated,
  );
});
