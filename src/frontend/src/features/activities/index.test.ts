import { ActivityResourceType } from '@/api/generated/api.types';
import { getActivityResourceOptions } from './index';

describe('activity resource options', () => {
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
