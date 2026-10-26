vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => String }));

import {
  ActivityEventType,
  ActivityResourceType,
  ActivityStatus,
  AuthorizedGitRepositoryView,
  GitRepositoryStatus,
  GitRepositoryRefStatus,
  GitRepositorySyncMode,
  ResourceControlState,
  LatestActivityView,
} from '@/api/generated/api.types';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GitRepoFormComponents } from '.';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: () => null,
  MonacoToArrayEditor: () => null,
  MonacoDiff: () => null,
}));

it('shows repository processing from enqueue through execution, then the final status', () => {
  const Indicator = GitRepoFormComponents.EditForm!.Header.Indicator;
  const repository = {
    status: GitRepositoryStatus.Pending,
    // Rust queues synchronization before a worker claims it.
    controlState: 'Queued',
  } as unknown as AuthorizedGitRepositoryView;
  const { rerender } = render(<Indicator resource={repository} />);
  expect(screen.getByText('Processing')).toHaveAttribute('data-slot', 'badge');
  expect(screen.queryByText('Pending')).not.toBeInTheDocument();

  rerender(<Indicator resource={{ ...repository, controlState: ResourceControlState.Processing }} />);
  expect(screen.getByText('Processing')).toBeVisible();

  rerender(
    <Indicator
      resource={{ ...repository, controlState: ResourceControlState.Idle, status: GitRepositoryStatus.Healthy }}
    />,
  );
  expect(screen.getByText('Healthy')).toBeVisible();
  expect(screen.queryByText('Processing')).not.toBeInTheDocument();

  rerender(
    <Indicator
      resource={{ ...repository, controlState: ResourceControlState.Idle, status: GitRepositoryStatus.Degraded }}
    />,
  );
  expect(screen.getByText('Degraded')).toBeVisible();
});

function renderSubHeader(
  latestActivityView: LatestActivityView | null,
  syncStatus: AuthorizedGitRepositoryView['syncStatus'] = null,
) {
  const SubHeader = GitRepoFormComponents.EditForm!.SubHeader!;
  return render(
    <SubHeader
      resource={
        {
          latestActivityView,
          syncStatus,
          defaultBranch: 'main',
          syncMode: GitRepositorySyncMode.PullInterval,
          syncIntervalMinutes: 5,
        } as AuthorizedGitRepositoryView
      }
    />,
  );
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
  it('clears a previous sync error when a realtime update reports recovery', () => {
    const SubHeader = GitRepoFormComponents.EditForm!.SubHeader!;
    const latestActivityView = activity(ActivityEventType.GitRepoPulled, {
      status: ActivityStatus.Failure,
      info: {
        $type: 'GitRepoPulled',
        result: { commitSha: null, message: 'Repository credentials were rejected.' },
      } as LatestActivityView['info'],
    });
    const resource = { latestActivityView, status: GitRepositoryStatus.Degraded } as AuthorizedGitRepositoryView;
    const { rerender } = render(<SubHeader resource={resource} />);
    expect(screen.getByText('Repository credentials were rejected.')).toBeVisible();

    rerender(<SubHeader resource={{ ...resource, status: GitRepositoryStatus.Healthy }} />);
    expect(screen.queryByText('Sync Error')).not.toBeInTheDocument();
    expect(screen.queryByText('Repository credentials were rejected.')).not.toBeInTheDocument();

    rerender(<SubHeader resource={resource} />);
    expect(screen.getByText('Sync Error')).toBeVisible();
  });

  it('preserves warnings on a healthy repository', () => {
    const SubHeader = GitRepoFormComponents.EditForm!.SubHeader!;
    const latestActivityView = activity(ActivityEventType.GitRepoPulled, {
      status: ActivityStatus.Warning,
      info: {
        $type: 'GitRepoPulled',
        result: { commitSha: null, message: 'Post-sync command reported a warning.' },
      } as LatestActivityView['info'],
    });
    render(
      <SubHeader
        resource={{ latestActivityView, status: GitRepositoryStatus.Healthy } as AuthorizedGitRepositoryView}
      />,
    );
    expect(screen.getByText('Post-sync command reported a warning.')).toBeVisible();
  });

  it('renders without a latest activity', () => {
    renderSubHeader(null);
    expect(screen.getByRole('button', { name: 'Repository check details' })).toBeVisible();
    expect(screen.queryByText('Last update activity')).not.toBeInTheDocument();
  });

  it.each([
    ActivityEventType.GitRepoCreated,
    ActivityEventType.GitRepoUpdated,
    ActivityEventType.GitRepoRenamed,
    ActivityEventType.GitRepoWebhookReceived,
  ])('does not treat %s as a successful synchronization', (eventType) => {
    renderSubHeader(activity(eventType));
    expect(screen.queryByText('Last update activity')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Repository check details' })).toBeVisible();
  });

  it.each([ActivityEventType.GitRepoCloned, ActivityEventType.GitRepoPulled])(
    'shows the commit for a successful %s',
    async (eventType) => {
      const user = userEvent.setup();
      const latestActivity = activity(eventType, {
        info: {
          $type: eventType,
          gitRepo: { id: 'repo-1', name: 'Example' },
          result: { commitSha: '0123456789abcdef0123456789abcdef01234567', message: null },
        } as LatestActivityView['info'],
      });
      const { rerender } = renderSubHeader(latestActivity, {
        status: GitRepositoryRefStatus.Healthy,
        lastSyncedAt: '2026-09-06T05:00:00Z',
      });

      expect(screen.getByText('Last update activity')).toBeVisible();
      expect(screen.queryByText('Last checked')).not.toBeInTheDocument();
      const trigger = screen.getByRole('button', { name: 'Repository check details' });
      await user.click(trigger);
      const details = screen.getByRole('dialog', { name: 'Repository checks' });
      expect(within(details).getByText('Last checked')).toBeVisible();
      expect(within(details).getByText('Every 5 minutes')).toBeVisible();
      expect(screen.getByText('Successful background syncs do not create activity entries.')).toBeVisible();
      expect(details.querySelector('time')).toHaveAttribute('dateTime', '2026-09-06T05:00:00.000Z');
      expect(screen.getByText(/Synced to commit/)).toBeVisible();
      expect(screen.getByText(/0123456789ab/)).toBeVisible();
      // A repository update refreshes the open popover without an API/query provider.
      const SubHeader = GitRepoFormComponents.EditForm!.SubHeader!;
      rerender(
        <SubHeader
          resource={
            {
              latestActivityView: latestActivity,
              defaultBranch: 'main',
              syncMode: GitRepositorySyncMode.PullInterval,
              syncIntervalMinutes: 5,
              syncStatus: { status: GitRepositoryRefStatus.Healthy, lastSyncedAt: '2026-09-06T05:05:00Z' },
            } as AuthorizedGitRepositoryView
          }
        />,
      );
      expect(details.querySelector('time')).toHaveAttribute('dateTime', '2026-09-06T05:05:00.000Z');

      await user.keyboard('{Escape}');
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
      expect(trigger).toHaveFocus();
    },
  );

  it.each([undefined, { commitSha: null }, { commitSha: '' }])(
    'does not crash when a successful sync has no commit (%j)',
    (result) => {
      const view = activity(ActivityEventType.GitRepoCloned, {
        info: { $type: 'GitRepoCloned', result } as LatestActivityView['info'],
      });
      renderSubHeader(view);
      expect(screen.queryByText('Last update activity')).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Repository check details' })).toBeVisible();
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
    expect(screen.queryByText('Last update activity')).not.toBeInTheDocument();
  });
});
