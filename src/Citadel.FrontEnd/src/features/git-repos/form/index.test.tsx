import {
  ActivityEventType,
  ActivityResourceType,
  ActivityStatus,
  GitRepositoryView,
  LatestActivityView,
} from '@/api/generated/api.types';
import { render, screen } from '@testing-library/react';
import { GitRepoFormComponents } from '.';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: () => null,
  MonacoToArrayEditor: () => null,
  MonacoDiff: () => null,
}));

function renderSubHeader(latestActivityView: LatestActivityView | null) {
  const SubHeader = GitRepoFormComponents.EditForm!.SubHeader!;
  return render(<SubHeader resource={{ latestActivityView } as GitRepositoryView} />);
}

function activity(eventType: ActivityEventType, overrides: Partial<LatestActivityView> = {}): LatestActivityView {
  return {
    id: 'activity-1',
    resourceType: ActivityResourceType.GitRepository,
    eventType,
    status: ActivityStatus.Success,
    createdAt: '2026-09-05T12:00:00Z',
    info: { $type: eventType } as LatestActivityView['info'],
    ...overrides,
  };
}

describe('Git repository subheader', () => {
  it('renders without a latest activity', () => {
    expect(renderSubHeader(null).container).toBeEmptyDOMElement();
  });

  it.each([
    ActivityEventType.GitRepoCreated,
    ActivityEventType.GitRepoUpdated,
    ActivityEventType.GitRepoRenamed,
    ActivityEventType.GitRepoWebhookReceived,
  ])('does not treat %s as a successful synchronization', (eventType) => {
    expect(renderSubHeader(activity(eventType)).container).toBeEmptyDOMElement();
  });

  it.each([ActivityEventType.GitRepoCloned, ActivityEventType.GitRepoPulled])(
    'shows the commit for a successful %s',
    (eventType) => {
      renderSubHeader(
        activity(eventType, {
          info: {
            $type: eventType,
            gitRepo: { id: 'repo-1', name: 'Example' },
            result: { commitSha: '0123456789abcdef0123456789abcdef01234567', message: null },
          } as LatestActivityView['info'],
        }),
      );

      expect(screen.getByText(/Repository updated successfully to commit/)).toBeVisible();
      expect(screen.getByText(/0123456789ab/)).toBeVisible();
    },
  );

  it.each([undefined, { commitSha: null }, { commitSha: '' }])(
    'does not crash when a successful sync has no commit (%j)',
    (result) => {
      const view = activity(ActivityEventType.GitRepoCloned, {
        info: { $type: 'GitRepoCloned', result } as LatestActivityView['info'],
      });
      expect(renderSubHeader(view).container).toBeEmptyDOMElement();
    },
  );

  it.each([ActivityStatus.Failure, ActivityStatus.Warning])('preserves %s synchronization details', (status) => {
    renderSubHeader(
      activity(ActivityEventType.GitRepoPulled, {
        status,
        info: {
          $type: 'GitRepoPulled',
          result: { commitSha: null, message: 'Repository credentials were rejected.' },
        } as LatestActivityView['info'],
      }),
    );

    expect(screen.getByText('Sync Error')).toBeVisible();
    expect(screen.getByText('Repository credentials were rejected.')).toBeVisible();
    expect(screen.queryByText(/Repository updated successfully/)).not.toBeInTheDocument();
  });
});
