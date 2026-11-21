import { screen, waitFor } from '@testing-library/react';
import { useLocation } from 'react-router';
import { useSelectedResources } from '@/lib/atoms';
import { renderCitadel } from '@/test/render-citadel';
import { useRemoveResourceAction } from './useSettledResourceAction';

it.each([
  ['ok', '/scripts'],
  ['failed', '/scripts/edit/failed'],
])('keeps failed selections and leaves the correct detail open for %s', async (id, expectedPath) => {
  const reconcile = vi.fn();
  const items = [{ id: 'ok' }, { id: 'failed' }];
  function Probe() {
    const [selected, setSelected] = useSelectedResources<{ id: string }>('Script');
    const location = useLocation();
    const action = useRemoveResourceAction(items, {
      type: 'Script',
      listPath: '/scripts',
      mutate: async (item) => {
        if (item.id === 'failed') throw Error('fixture failure');
      },
      onRemoved: reconcile,
      message: () => 'One script deleted',
    });
    return (
      <>
        <button onClick={() => setSelected(items)}>Select</button>
        <button onClick={action.run}>Delete</button>
        <output>{selected.map((item) => item.id).join(',')}</output>
        <span>{location.pathname}</span>
      </>
    );
  }
  const { user } = renderCitadel(<Probe />, { route: `/scripts/edit/${id}` });
  await user.click(screen.getByRole('button', { name: 'Select' }));
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('failed'));
  expect(screen.getByRole('status')).not.toHaveTextContent('ok');
  expect(screen.getByText(expectedPath)).toBeVisible();
  expect(reconcile).toHaveBeenCalledOnce();
});
