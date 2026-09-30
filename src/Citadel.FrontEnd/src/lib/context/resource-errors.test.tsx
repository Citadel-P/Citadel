import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { ActivityStatus, BuildRunStatus, StackReleaseStatus, ResourceControlState } from '@/api/generated/api.types';
import { GitRepoFormComponents } from '@/features/git-repos/form';
import { DeploymentFormComponents } from '@/features/deployments/form';
import { StackFormComponents } from '@/features/stacks/form';
import { BuildFormComponents } from '@/features/builds/form';
import { createStack } from '@/test/factories/resources';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';

const id = '00000000-0000-0000-0000-000000000001';
const message = 'Unable to authenticate with the remote repository.';
const resources = [
  {
    name: 'Git repository',
    forms: GitRepoFormComponents,
    endpoint: 'gitRepositories',
    event: 'GitRepositoryInfoUpdated',
    activity: 'GitRepoCloned',
    initial: { id, latestActivityView: null },
  },
  {
    name: 'Deployment',
    forms: DeploymentFormComponents,
    endpoint: 'deployments',
    event: 'DeploymentInfoUpdated',
    activity: 'DeploymentApplied',
    initial: { id, latestActivityView: null, spec: { updateBehavior: 'Disabled' } },
  },
  {
    name: 'Stack',
    forms: StackFormComponents,
    endpoint: 'stacks',
    event: 'StackInfoUpdated',
    activity: 'StackApplied',
    initial: createStack({ latestActivityView: null, status: StackReleaseStatus.Failed }),
  },
];

for (const resource of resources) {
  describe(`${resource.name} live error feedback`, () => {
    it.each(['JSON', 'tuple'])('shows the first %s failure beneath the actions without a reload', async (format) => {
      const fake = new FakeRealtimeConnection();
      const read = vi.fn(() => HttpResponse.json(resource.initial));
      server.use(http.get(`http://localhost/api/v1/${resource.endpoint}/${id}`, read));
      const useData = resource.forms.EditForm!.useData!;
      const SubHeader = resource.forms.EditForm!.SubHeader!;
      function Probe() {
        const { item } = useData(id);
        return item ? (
          <>
            <button>Actions</button>
            <SubHeader resource={item as any} />
          </>
        ) : null;
      }
      renderCitadel(<Probe />, {
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      });
      await screen.findByRole('button', { name: 'Actions' });
      await waitFor(() => expect(fake.listenerCount(resource.event)).toBe(1));
      expect(screen.queryByText(message)).not.toBeInTheDocument();
      const info = { result: { message, code: 500 } };
      const event = {
        ...resource.initial,
        latestActivityView: {
          id: 'activity-id',
          createdAt: '2026-09-22T00:00:00Z',
          status: ActivityStatus.Failure,
          info: format === 'JSON' ? { ...info, $type: resource.activity } : [resource.activity, info],
        },
      };
      const original = structuredClone(event);
      act(() => fake.emit(resource.event, { ...event, id: 'different-resource' }));
      expect(screen.queryByText(message)).not.toBeInTheDocument();
      act(() => fake.emit(resource.event, event));
      expect(await screen.findByText(message)).toBeVisible();
      expect(event).toEqual(original);
      expect(read).toHaveBeenCalledTimes(1);
      act(() =>
        fake.emit(resource.event, {
          ...event,
          latestActivityView: { ...event.latestActivityView, status: ActivityStatus.Success },
        }),
      );
      await waitFor(() => expect(screen.queryByText(message)).not.toBeInTheDocument());
    });
  });
}

it.each([BuildRunStatus.Failed, BuildRunStatus.TimedOut, BuildRunStatus.Interrupted])(
  'shows a live %s build error and clears it for a new run',
  async (status) => {
    const fake = new FakeRealtimeConnection();
    const initial = { id, enabled: true, currentRunId: null, controlState: ResourceControlState.Idle, latestRun: null };
    const read = vi.fn(() => HttpResponse.json(initial));
    server.use(http.get(`http://localhost/api/v1/buildProjects/${id}`, read));
    const useData = BuildFormComponents.EditForm!.useData!;
    const SubHeader = BuildFormComponents.EditForm!.SubHeader!;
    function Probe() {
      const { item } = useData(id);
      return item ? (
        <>
          <button>Actions</button>
          <SubHeader resource={item as any} />
        </>
      ) : null;
    }
    renderCitadel(<Probe />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    await screen.findByRole('button', { name: 'Actions' });
    await waitFor(() => expect(fake.listenerCount('BuildProjectInfoUpdated')).toBe(1));
    const latestRun = { id: 'run-id', status, errorMessage: message };
    act(() => fake.emit('BuildProjectInfoUpdated', { ...initial, latestRun }));
    expect(await screen.findByText(message)).toBeVisible();
    expect(read).toHaveBeenCalledTimes(1);
    act(() =>
      fake.emit('BuildProjectInfoUpdated', {
        ...initial,
        latestRun: { ...latestRun, id: 'new-run', status: BuildRunStatus.Running, errorMessage: null },
      }),
    );
    await waitFor(() => expect(screen.queryByText(message)).not.toBeInTheDocument());
  },
);
