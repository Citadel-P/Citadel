import { render, screen } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { MemoryRouter } from 'react-router';
import { NetworkViewControls, NetworkViews } from './views';

const state = vi.hoisted(() => ({ loadMap: vi.fn() }));
vi.mock('./map/network-map', () => {
  state.loadMap();
  return { default: () => <div>Focused map</div> };
});
vi.mock('./table', () => ({ NetworksTable: () => <div>Network list</div> }));

it('keeps the table and Columns visible without importing the graph, including an old Map preference', () => {
  localStorage.setItem('networks-view', JSON.stringify('map'));
  render(
    <Provider store={createStore()}>
      <MemoryRouter initialEntries={['/?view=map']}>
        <NetworkViewControls />
        <NetworkViews items={[]} actions={{}} isLoading={false} />
      </MemoryRouter>
    </Provider>,
  );
  expect(screen.getByText('Network list')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Columns' })).toBeVisible();
  expect(screen.queryByRole('tab', { name: 'Map' })).not.toBeInTheDocument();
  expect(state.loadMap).not.toHaveBeenCalled();
});
