import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { Provider } from 'jotai';
import { MemoryRouter, useSearchParams } from 'react-router';
import { PlatformStatus, WorkloadHealthStatus } from '@/api/generated/api.types';
import { createWorkloadHealth } from '@/test/factories/resources';
import { WorkloadHealthColumns, WorkloadHealthFilter, useWorkloadHealthFilter } from './table';
const items = [
  {
    id: 'owner',
    name: 'web',
    platformStatus: PlatformStatus.Online,
    probeHealth: createWorkloadHealth({ status: WorkloadHealthStatus.Healthy, observedAt: Date.now() }),
  },
];
function Fixture() {
  const result = useWorkloadHealthFilter(items, 'Deployment');
  const [params] = useSearchParams();
  return (
    <>
      <WorkloadHealthFilter kind="Deployment" />
      <WorkloadHealthColumns kind="Deployment" />
      <output aria-label="count">{result.length}</output>
      <output aria-label="selection">{params.get('health')}</output>
    </>
  );
}
describe('health columns', () => {
  it('hides and pauses the filter while preserving its URL selection', async () => {
    render(
      <Provider>
        <MemoryRouter initialEntries={['/?health=Unhealthy']}>
          <Fixture />
        </MemoryRouter>
      </Provider>,
    );
    expect(screen.getByLabelText('count')).toHaveTextContent('0');
    expect(screen.getByRole('button', { name: 'Health Filter' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Columns' }));
    await userEvent.click(screen.getByRole('menuitemcheckbox', { name: 'Health' }));
    expect(screen.queryByRole('button', { name: 'Health Filter' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('count')).toHaveTextContent('1');
    expect(screen.getByLabelText('selection')).toHaveTextContent('Unhealthy');
  });
});
