import { screen } from '@testing-library/react';
import { SwarmConfigView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { canDeleteConfig } from './actions';
import { ConfigsTable } from './table';

vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => (value: string) => value }));

const createConfig = (inUse: boolean, isStale = false): SwarmConfigView => ({
  id: inUse ? 'used-config' : 'unused-config',
  versionIndex: 1,
  name: inUse ? 'Used config' : 'Unused config',
  templatingDriver: null,
  serviceNames: inUse ? ['web'] : [],
  labels: {},
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T00:00:00Z',
  isStale,
  inUse,
  capabilities: null,
});

describe('ConfigsTable', () => {
  it('shows selection checkboxes and volume-style usage indicators', () => {
    renderCitadel(<ConfigsTable items={[createConfig(true), createConfig(false)]} actions={{}} isLoading={false} />, {
      route: '/platforms/platform-1/configs',
    });

    expect(screen.getAllByRole('checkbox')).toHaveLength(3);
    expect(screen.getByRole('link', { name: 'Used config' }).previousElementSibling).toHaveClass('bg-green-500');
    expect(screen.getByRole('link', { name: 'Unused config' }).previousElementSibling).toHaveClass('bg-gray-500');
  });

  it('allows deletion only when the projection is current and unused', () => {
    expect(canDeleteConfig(createConfig(false))).toBe(true);
    expect(canDeleteConfig(createConfig(true))).toBe(false);
    expect(canDeleteConfig(createConfig(false, true))).toBe(false);
  });
});
