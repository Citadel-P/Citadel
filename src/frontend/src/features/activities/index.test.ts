import { ActivityResourceType } from '@/api/generated/api.types';
import { getActivityEventOptions, getActivityResourceOptions } from './index';

describe('activity resource options', () => {
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
