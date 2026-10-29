import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FileSystemPlatformSelector, SecretSelector } from './form';

it('keeps repository scoping and searches stored secrets', async () => {
  server.use(
    http.get('http://localhost/api/v1/resourceBindings/secrets', ({ request }) => {
      const query = new URL(request.url).searchParams;
      expect(query.get('targetResourceType')).toBe('BackupRepository');
      expect(query.get('targetResourceId')).toBe('repository');
      return HttpResponse.json({ secrets: [{ id: 'password', name: 'Restic password' }] });
    }),
  );
  const onChange = vi.fn();
  const { user } = renderCitadel(<SecretSelector targetResourceId="repository" onChange={onChange} />);
  await screen.findByText('Select a stored secret');
  await user.click(screen.getByRole('combobox'));
  expect(screen.queryByRole('option', { name: 'No session token' })).not.toBeInTheDocument();
  await user.type(screen.getByPlaceholderText('Search secrets...'), 'Restic');
  await user.click(screen.getByRole('option', { name: 'Restic password' }));
  expect(onChange).toHaveBeenCalledWith('password');
});

it('clears an optional session token while displaying a saved unavailable secret', async () => {
  server.use(http.get('http://localhost/api/v1/resourceBindings/secrets', () => HttpResponse.json({ secrets: [] })));
  const onChange = vi.fn();
  const { user } = renderCitadel(<SecretSelector allowNone value="saved-token" onChange={onChange} />);
  const trigger = screen.getByRole('combobox');
  expect(trigger).toHaveTextContent('saved-token');
  await vi.waitFor(() => expect(trigger).toBeEnabled());
  await user.click(trigger);
  await user.click(screen.getByRole('option', { name: 'No session token' }));
  expect(onChange).toHaveBeenCalledWith(null);
});

it('explains where to create a required secret when none are available', async () => {
  server.use(http.get('http://localhost/api/v1/resourceBindings/secrets', () => HttpResponse.json({ secrets: [] })));
  const { user } = renderCitadel(<SecretSelector onChange={() => {}} />);
  await screen.findByText('Select a stored secret');
  await user.click(screen.getByRole('combobox'));
  expect(screen.getByText('No stored secrets found. Create one from Bindings first.')).toBeVisible();
});

it('offers Local, Direct and Edge Docker hosts, excluding Swarm', async () => {
  server.use(
    http.get('http://localhost/api/v1/platforms', () =>
      HttpResponse.json({
        platforms: [
          { id: 'local', name: 'Local host', connectorType: 'Local', platformDescriptor: { $type: 'Docker' } },
          { id: 'direct', name: 'Direct host', connectorType: 'Agent', platformDescriptor: { $type: 'Docker' } },
          { id: 'swarm', name: 'Swarm host', platformDescriptor: { $type: 'DockerSwarm' } },
          { id: 'edge', name: 'Edge host', connectorType: 'EdgeAgent', platformDescriptor: { $type: 'Docker' } },
        ],
      }),
    ),
  );
  const onSelect = vi.fn();
  const { user } = renderCitadel(<FileSystemPlatformSelector onSelect={onSelect} />);
  await screen.findByText('Select Platform');
  await user.click(screen.getByRole('combobox'));
  expect(screen.getByRole('option', { name: 'Direct host' })).toBeVisible();
  expect(screen.getByRole('option', { name: 'Edge host' })).toBeVisible();
  expect(screen.queryByRole('option', { name: 'Swarm host' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('option', { name: 'Edge host' }));
  expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ id: 'edge' }));
});
