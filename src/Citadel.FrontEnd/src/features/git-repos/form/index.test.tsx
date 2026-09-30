import {
  ActivityEventType,
  ActivityResourceType,
  ActivityStatus,
  AuthorizedGitRepositoryView,
  GitRepositoryStatus,
  ResourceControlState,
  LatestActivityView,
} from '@/api/generated/api.types';
import { render, screen } from '@testing-library/react';
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
    <Indicator resource={{ ...repository, controlState: ResourceControlState.Idle, status: GitRepositoryStatus.Healthy }} />,
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

function renderSubHeader(latestActivityView: LatestActivityView | null) {
  const SubHeader = GitRepoFormComponents.EditForm!.SubHeader!;
  return render(<SubHeader resource={{ latestActivityView } as AuthorizedGitRepositoryView} />);
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
    render(<SubHeader resource={{ latestActivityView, status: GitRepositoryStatus.Healthy } as AuthorizedGitRepositoryView} />);
    expect(screen.getByText('Post-sync command reported a warning.')).toBeVisible();
  });

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
