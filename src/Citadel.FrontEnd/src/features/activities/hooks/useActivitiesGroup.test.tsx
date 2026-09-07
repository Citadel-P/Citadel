import { renderHook } from '@testing-library/react';
import { useActivitiesGroup } from './useActivitiesGroup';

const mocks = vi.hoisted(() => ({
  useRead: vi.fn(() => ({ data: undefined, isLoading: false })),
}));

vi.mock('@/lib/hooks', () => ({ useRead: mocks.useRead }));
vi.mock('@/lib/atoms', () => ({
  useActivityQuery: () => [
    {
      page: 1,
      pageSize: 25,
      resourceType: 'All',
      resourceId: undefined,
      eventType: 'All',
    },
  ],
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: vi.fn() }));

describe('useActivitiesGroup', () => {
  it('passes the resource tab type to the activities endpoint', () => {
    renderHook(() => useActivitiesGroup('team-id', 'Team', 20));

    expect(mocks.useRead).toHaveBeenCalledWith('listActivities', {
      query: {
        Page: 1,
        PageSize: 20,
        ResourceType: 'Team',
        EventType: undefined,
        ResourceId: 'team-id',
      },
    });
  });
});
