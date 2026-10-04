import type { ActivityView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { TaskSheet } from './task-sheet';

const mocks = vi.hoisted(() => ({
  activity: {
    id: '019ffb55-f35e-7178-91f2-281696860dc8',
    platformId: null,
    resourceId: '019ffb55-f35e-7178-91f2-281696860dc7',
    platformName: '',
    resourceName: 'build-runner',
    platformStatus: 'Offline',
    resourceType: 'ServiceAccount',
    eventType: 'ServiceAccountRenamed',
    status: 'Success',
    createdAt: '2026-08-13T12:00:00Z',
    info: {
      $type: 'ServiceAccountRenamed',
      oldName: 'old-runner',
      newName: 'build-runner',
    },
    actorId: '019ffb55-f35e-7178-91f2-281696860dc9',
    actorName: 'Administrator',
    actorType: 'User',
  } as ActivityView,
}));

vi.mock('@/lib/atoms', () => ({
  useTaskSheet: () => ({
    state: {
      open: true,
      task: { kind: 'activity', payload: { id: mocks.activity.id } },
    },
    close: vi.fn(),
  }),
}));

vi.mock('@/lib/hooks', () => ({
  useRead: () => ({ data: { data: mocks.activity }, isLoading: false }),
}));

vi.mock('@/lib/monaco', () => ({
  MonacoDiff: () => <div data-testid="monaco-diff" />,
  MonacoEditor: ({ title, value }: { title: string; value: string }) => (
    <div data-testid="monaco-editor">
      <h3>{title}</h3>
      <pre>{value}</pre>
    </div>
  ),
}));

describe('TaskSheet activity details', () => {
  it('renders a Service Account rename as a concise message instead of Monaco', () => {
    renderCitadel(<TaskSheet type="Activity" />);

    expect(screen.getByText('old-runner').parentElement).toHaveTextContent(
      'Service Account renamed from old-runner to build-runner.',
    );
    expect(screen.getByText('build-runner', { selector: 'b' })).toBeVisible();
    expect(screen.queryByTestId('monaco-editor')).not.toBeInTheDocument();
    expect(screen.queryByTestId('monaco-diff')).not.toBeInTheDocument();
  });

  it.each([
    ['User', 'User renamed from old-name to new-name.'],
    ['Team', 'Team renamed from old-name to new-name.'],
    ['Role', 'Role renamed from old-name to new-name.'],
  ] as const)('renders a %s rename as a concise message', (resourceType, expected) => {
    mocks.activity = {
      ...mocks.activity,
      resourceType,
      eventType: `${resourceType}Renamed`,
      info: {
        $type: `${resourceType}Renamed`,
        oldName: 'old-name',
        newName: 'new-name',
      },
    } as ActivityView;

    renderCitadel(<TaskSheet type="Activity" />);

    expect(screen.getByText('old-name').parentElement).toHaveTextContent(expected);
    expect(screen.queryByTestId('monaco-editor')).not.toBeInTheDocument();
    expect(screen.queryByTestId('monaco-diff')).not.toBeInTheDocument();
  });

  it.each([
    ['ActionWebhookReceived', 'AutomationAction'],
    ['BackupPolicyWebhookReceived', 'BackupPolicy'],
  ] as const)('shows delivery details for %s', (eventType, resourceType) => {
    mocks.activity = {
      ...mocks.activity,
      resourceType,
      eventType,
      status: 'Warning',
      info: {
        $type: eventType,
        requestId: 'webhook-request',
        authType: 'generic',
        execution: 'run',
        status: 'noop',
        reason: 'Automated operations require an active license entitlement.',
      },
    } as ActivityView;

    renderCitadel(<TaskSheet type="Activity" />);

    expect(screen.getByText('Webhook details')).toBeVisible();
    expect(screen.getByTestId('monaco-editor')).toHaveTextContent(
      'Automated operations require an active license entitlement.',
    );
    expect(screen.getByTestId('monaco-editor')).toHaveTextContent('webhook-request');
  });

  it('shows the backup run, trigger, duration and failure details', () => {
    mocks.activity = {
      ...mocks.activity,
      resourceType: 'BackupPolicy',
      eventType: 'BackupRunCompleted',
      status: 'Failure',
      info: {
        $type: 'BackupRunCompleted',
        runId: '019ffb55-f35e-7178-91f2-281696860dca',
        trigger: 'Schedule',
        status: 'Failed',
        durationMs: 1250,
        errorMessage: 'Backup repository unavailable.',
      },
    } as ActivityView;

    renderCitadel(<TaskSheet type="Activity" />);

    expect(screen.getByText('019ffb55-f35e-7178-91f2-281696860dca')).toBeVisible();
    expect(screen.getByText('Schedule')).toBeVisible();
    expect(screen.getByText('1250 ms')).toBeVisible();
    expect(screen.getByText('Backup repository unavailable.')).toBeVisible();
    expect(screen.queryByTestId('monaco-editor')).not.toBeInTheDocument();
  });
});

describe('Stack activity container IDs', () => {
  it.each(['StackApplied', 'StackRollback'] as const)('hides missing IDs and displays captured IDs for %s', (type) => {
    mocks.activity = {
      ...mocks.activity,
      resourceType: 'Stack',
      eventType: type,
      status: 'Success',
      info:
        type === 'StackApplied'
          ? { $type: type, stack: null, result: { containerIds: null } }
          : { $type: type, oldStack: null, newStack: null, result: { containerIds: null } },
    } as ActivityView;
    const { unmount } = renderCitadel(<TaskSheet type="Activity" />);
    expect(screen.queryByText(/^Container IDs?$/)).not.toBeInTheDocument();
    mocks.activity = {
      ...mocks.activity,
      info:
        type === 'StackApplied'
          ? { $type: type, stack: null, result: { containerIds: ['docker-web', 'docker-worker'] } }
          : { $type: type, oldStack: null, newStack: null, result: { containerIds: ['docker-web', 'docker-worker'] } },
    };
    unmount();
    renderCitadel(<TaskSheet type="Activity" />);
    expect(screen.getByText('Container IDs')).toBeVisible();
    expect(screen.getByText('docker-web')).toBeVisible();
    expect(screen.getByText('docker-worker')).toBeVisible();
  });
});

describe('Failed stack activity context', () => {
  it.each(['StackApplied', 'StackRollback'] as const)('shows the attempted configuration and error for %s', (type) => {
    const snapshot = {
      id: 'stack',
      name: 'web-stack',
      stackSource: 'WebEditor',
      stackRelease: { spec: { composeFile: 'services: web' } },
    };
    mocks.activity = {
      ...mocks.activity,
      resourceType: 'Stack',
      eventType: type,
      status: 'Failure',
      info:
        type === 'StackApplied'
          ? { $type: type, stack: snapshot, result: { message: 'Docker: port 8080 is already allocated' } }
          : {
              $type: type,
              oldStack: null,
              newStack: snapshot,
              result: { message: 'Docker: port 8080 is already allocated' },
            },
    } as ActivityView;
    renderCitadel(<TaskSheet type="Activity" />);
    expect(
      screen.getByText(type === 'StackApplied' ? 'Attempted configuration' : 'Attempted rollback configuration'),
    ).toBeVisible();
    expect(screen.getByTestId('monaco-editor')).toHaveTextContent('services: web');
    expect(screen.getByText('Docker: port 8080 is already allocated')).toBeVisible();
    expect(screen.queryByText('Applied configuration')).not.toBeInTheDocument();
  });
  it('does not render a null configuration for an older failure', () => {
    mocks.activity = {
      ...mocks.activity,
      eventType: 'StackApplied',
      status: 'Failure',
      info: { $type: 'StackApplied', stack: null, result: { message: 'Stack deployment failed.' } },
    } as ActivityView;
    renderCitadel(<TaskSheet type="Activity" />);
    expect(screen.queryByTestId('monaco-editor')).not.toBeInTheDocument();
    expect(screen.getByText('Stack deployment failed.')).toBeVisible();
  });
});
