import type { ActivityView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { TaskSheet } from './task-sheet';

const mocks = vi.hoisted(() => ({
  activity: {
    id: '019ffb55-f35e-7178-91f2-281696860dc8',
    platformId: null,
    resourceId: '019ffb55-f35e-7178-91f2-281696860dc7',
    platformName: null,
    resourceName: 'build-runner',
    platformStatus: null,
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
  MonacoEditor: () => <div data-testid="monaco-editor" />,
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
});
