import { act, renderHook } from '@testing-library/react';
import { useActivitiesGroup } from './useActivitiesGroup';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { ActivityView, PagedActivityView } from '@/api/generated/api.types';
import { RealtimeConnection } from '@/lib/realtime-connection';

const mocks = vi.hoisted(() => ({
  useRead: vi.fn(() => ({
    data: undefined as { data: { pagedResult: PagedActivityView } } | undefined,
    isLoading: false,
  })),
  eventType: 'All',
}));

vi.mock('@/lib/hooks', () => ({ useRead: mocks.useRead }));
vi.mock('@/lib/atoms', () => ({
  useActivityQuery: () => [
    {
      page: 1,
      pageSize: 25,
      resourceType: 'All',
      resourceId: undefined,
      eventType: mocks.eventType,
    },
  ],
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: vi.fn() }));

describe('useActivitiesGroup', () => {
  beforeEach(() => {
    mocks.eventType = 'All';
    mocks.useRead.mockReset();
    mocks.useRead.mockReturnValue({ data: undefined, isLoading: false });
  });
  it('passes the resource tab type to the activities endpoint', () => {
    renderHook(() => useActivitiesGroup('team-id', 'Team', 20));

    expect(mocks.useRead).toHaveBeenCalledWith('listActivities', {
      query: {
        Page: 1,
        PageSize: 20,
        ResourceType: 'Team',
        EventType: undefined,
        Status: undefined,
        ResourceId: 'team-id',
      },
    });
  });
  it('requests failed backup completion records instead of filtering a page of mixed results', () => {
    mocks.eventType = 'BackupRunFailed';
    renderHook(() => useActivitiesGroup());
    expect(mocks.useRead).toHaveBeenCalledWith('listActivities', {
      query: {
        Page: 1,
        PageSize: 25,
        ResourceType: undefined,
        ResourceId: undefined,
        EventType: 'BackupRunCompleted',
        Status: 'Failure',
      },
    });
  });
  it('keeps successful and unrelated realtime events out of the failed backup results', () => {
    mocks.eventType = 'BackupRunFailed';
    mocks.useRead.mockReturnValue({
      isLoading: false,
      data: { data: { pagedResult: { items: [], totalCount: 0, page: 1, pageSize: 25 } } },
    });
    const { result } = renderHook(() => useActivitiesGroup('policy', 'BackupPolicy'));
    const on = vi.fn();
    vi.mocked(useRealtimeGroup).mock.lastCall![0].setupEventListeners?.({ on } as unknown as RealtimeConnection);
    const receive = on.mock.calls.find(([name]) => name === 'ActivityEventReceived')![1];
    act(() => {
      for (const [id, eventType, status, resourceId] of [
        ['success', 'BackupRunCompleted', 'Success', 'policy'],
        ['cancelled', 'BackupRunCompleted', 'Warning', 'policy'],
        ['unrelated', 'BackupPolicyUpdated', 'Failure', 'policy'],
        ['other-policy', 'BackupRunCompleted', 'Failure', 'other-policy'],
        ['failed', 'BackupRunCompleted', 'Failure', 'policy'],
      ]) {
        receive({ id, eventType, status, resourceId, resourceType: 'BackupPolicy' } as ActivityView);
      }
    });
    expect(result.current.pagedActivities?.items.map((item) => item.id)).toEqual(['failed']);
    expect(result.current.pagedActivities?.totalCount).toBe(1);
  });
});
