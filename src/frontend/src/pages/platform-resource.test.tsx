import { PlatformType } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { Route, Routes } from 'react-router';
import { beforeEach, vi } from 'vitest';
import PlatformResourcePage from './platform-resource';

const mocks = vi.hoisted(() => ({
  useAppContext: vi.fn(),
}));

vi.mock('@/lib/context/app-context', () => ({ useAppContext: mocks.useAppContext }));
vi.mock('./resource-swarm', () => ({ default: () => <div>Swarm resource</div> }));
vi.mock('./resource', () => ({ default: () => <div>Docker resource list</div> }));
vi.mock('./resource-docker-info', () => ({ default: () => <div>Docker resource details</div> }));

describe('PlatformResourcePage', () => {
  beforeEach(() => mocks.useAppContext.mockReset());

  it('uses the flattened Swarm route for a Swarm platform', () => {
    mocks.useAppContext.mockReturnValue({
      currentPlatform: { type: PlatformType.DockerSwarm },
      isLoading: false,
    });

    renderPage('/platforms/swarm-1/nodes');

    expect(screen.getByText('Swarm resource')).toBeVisible();
  });

  it('keeps the same resource route available to Docker platforms', () => {
    mocks.useAppContext.mockReturnValue({
      currentPlatform: { type: PlatformType.Docker },
      isLoading: false,
    });

    renderPage('/platforms/docker-1/networks');

    expect(screen.getByText('Docker resource list')).toBeVisible();
  });

  it('uses the existing Docker network screen for a Swarm manager', () => {
    mocks.useAppContext.mockReturnValue({
      currentPlatform: { type: PlatformType.DockerSwarm },
      isLoading: false,
    });

    renderPage('/platforms/swarm-1/networks');

    expect(screen.getByText('Docker resource list')).toBeVisible();
    expect(screen.queryByText('Swarm resource')).not.toBeInTheDocument();
  });

  it('keeps Docker resource detail routing intact', () => {
    mocks.useAppContext.mockReturnValue({
      currentPlatform: { type: PlatformType.Docker },
      isLoading: false,
    });

    renderPage('/platforms/docker-1/containers/container-1');

    expect(screen.getByText('Docker resource details')).toBeVisible();
  });
});

const renderPage = (route: string) =>
  renderCitadel(
    <Routes>
      <Route path="/platforms/:platformId/:type/:resourceId?" element={<PlatformResourcePage />} />
    </Routes>,
    { route },
  );
