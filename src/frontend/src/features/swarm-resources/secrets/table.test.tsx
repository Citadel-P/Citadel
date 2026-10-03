import { screen } from '@testing-library/react';
import { SwarmSecretView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { canDeleteSecret } from './actions';
import { SecretsTable } from './table';

vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => (value: string) => value }));

const createSecret = (inUse: boolean, isStale = false): SwarmSecretView => ({
  id: inUse ? 'used-secret' : 'unused-secret',
  versionIndex: 1,
  name: inUse ? 'Used secret' : 'Unused secret',
  driver: null,
  serviceNames: inUse ? ['web'] : [],
  labels: {},
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T00:00:00Z',
  isStale,
  inUse,
  capabilities: null,
});

describe('SecretsTable', () => {
  it('shows selection checkboxes and volume-style usage indicators', () => {
    renderCitadel(<SecretsTable items={[createSecret(true), createSecret(false)]} actions={{}} isLoading={false} />, {
      route: '/platforms/platform-1/secrets',
    });

    expect(screen.getAllByRole('checkbox')).toHaveLength(3);
    expect(screen.getByRole('link', { name: 'Used secret' }).previousElementSibling).toHaveClass('bg-green-500');
    expect(screen.getByRole('link', { name: 'Unused secret' }).previousElementSibling).toHaveClass('bg-gray-500');
  });

  it('allows deletion only when the projection is current and unused', () => {
    expect(canDeleteSecret(createSecret(false))).toBe(true);
    expect(canDeleteSecret(createSecret(true))).toBe(false);
    expect(canDeleteSecret(createSecret(false, true))).toBe(false);
  });
});
