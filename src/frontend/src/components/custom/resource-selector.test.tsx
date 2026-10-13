import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { LookupResourceType } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ResourceSelectorField } from './common';

it('searches supplied nodes without a lookup and preserves an unavailable saved selection', async () => {
  const lookup = vi.fn(() => HttpResponse.json([]));
  server.use(http.get('http://localhost/api/v1/lookup', lookup));
  const onSelect = vi.fn();
  const { user } = renderCitadel(
    <ResourceSelectorField
      items={[
        { id: 'node-a', name: 'worker-a' },
        { id: 'node-b', name: 'worker-b' },
      ]}
      selected={{ id: 'saved-node', name: 'Unavailable worker' }}
      allowClear={false}
      searchPlaceholder="Search Nodes..."
      onSelect={onSelect}
    />,
  );
  const trigger = screen.getByRole('combobox');
  expect(trigger).toHaveTextContent('Unavailable worker');
  await user.click(trigger);
  expect(screen.queryByRole('option', { name: 'None' })).not.toBeInTheDocument();
  await user.type(screen.getByPlaceholderText('Search Nodes...'), 'worker-b');
  expect(screen.queryByRole('option', { name: 'worker-a' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('option', { name: 'worker-b' }));
  expect(onSelect).toHaveBeenCalledWith({ id: 'node-b', name: 'worker-b' });
  expect(lookup).not.toHaveBeenCalled();
});

it('still loads resources from the lookup API when no items are supplied', async () => {
  server.use(
    http.get('http://localhost/api/v1/lookup', ({ request }) => {
      expect(new URL(request.url).searchParams.get('TargetResourceType')).toBe('Platform');
      return HttpResponse.json([{ id: 'platform', name: 'Docker host' }]);
    }),
  );
  const onSelect = vi.fn();
  const { user } = renderCitadel(
    <ResourceSelectorField targetType={LookupResourceType.Platform} onSelect={onSelect} />,
  );
  await user.click(screen.getByRole('combobox'));
  await user.click(await screen.findByRole('option', { name: 'Docker host' }));
  expect(onSelect).toHaveBeenCalledWith({ id: 'platform', name: 'Docker host' });
});
