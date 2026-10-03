import { RegistryKind } from '@/api/generated/api.types';
import { registrySettings, registryUpdate } from './values';

describe('write-only registry credentials', () => {
  it('keeps presence flags out of form values and requests', () => {
    expect(registrySettings({ $type: 'DockerHub', userName: 'operator', hasPat: true })).toEqual({
      $type: 'DockerHub',
      userName: 'operator',
    });
    expect(registrySettings({ $type: 'Custom', authEnabled: true, hasPassword: true })).toEqual({
      $type: 'Custom',
      authEnabled: true,
    });
  });
  it('omits empty replacements but preserves explicit rotation and removal', () => {
    const value = { configuration: { $type: RegistryKind.Custom, password: '' } };
    expect(registryUpdate(value)).toEqual({ configuration: { $type: 'Custom' } });
    expect(value.configuration.password).toBe('');
    expect(registryUpdate({ configuration: { pat: 'replacement' } })).toEqual({
      configuration: { pat: 'replacement' },
    });
    expect(registryUpdate({ configuration: { password: null } })).toEqual({ configuration: { password: null } });
  });
});
