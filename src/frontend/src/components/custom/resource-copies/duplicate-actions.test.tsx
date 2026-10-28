import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { screen, waitFor } from '@testing-library/react';
import { useLocation } from 'react-router';
import { ResourceCopyEligibilityStatus, type ResourceCopyEligibility } from '@/api/generated/api.types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { renderCitadel } from '@/test/render-citadel';
import { DeploymentDropdownActions, DeploymentGroupActions } from '@/features/deployments/actions';
import { StackDropdownActions, StackGroupActions } from '@/features/stacks/actions';
import { SwarmServiceDropdownActions, SwarmServiceGroupActions } from '@/features/swarm-services/actions';

const source = {
  id: 'source',
  name: 'Application',
  platformId: 'platform',
  capabilities: { canRead: true, canWrite: true },
  copyEligibility: { status: ResourceCopyEligibilityStatus.Available, reason: null },
};
const workloads = [
  ['Deployment', DeploymentDropdownActions, DeploymentGroupActions, '/deployments'],
  ['Stack', StackDropdownActions, StackGroupActions, '/stacks'],
  ['SwarmService', SwarmServiceDropdownActions, SwarmServiceGroupActions, '/swarm-services'],
] as const;

function Location() {
  const { pathname, search } = useLocation();
  return (
    <output aria-label="Page URL">
      {pathname}
      {search}
    </output>
  );
}

// Render the exported actions used by each list, not a separate test-only menu.
describe.each(workloads)('%s duplicate actions', (kind, dropdown, group, path) => {
  it.each(['row', 'bar'] as const)('offers both copy modes from the %s menu', async (placement) => {
    const Group = group.duplicate;
    const { user } = renderCitadel(
      <>
        {placement === 'row' ? (
          <RowActionMenu<any> actions={{ duplicate: dropdown.duplicate }} resource={source as any} />
        ) : (
          <Group resources={[source] as any} />
        )}
        <Location />
      </>,
      { route: path },
    );
    async function open() {
      if (placement === 'row') {
        await user.click(screen.getByRole('button', { name: 'Open menu' }));
        const trigger = screen.getByRole('menuitem', { name: 'Duplicate' });
        trigger.focus();
        await user.keyboard('{ArrowRight}');
      } else await user.click(screen.getByRole('button', { name: 'Duplicate' }));
      expect(await screen.findByRole('menuitem', { name: /Configuration only/ })).toBeVisible();
      expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).not.toHaveAttribute(
        'aria-disabled',
        'true',
      );
    }
    await open();
    await user.click(screen.getByRole('menuitem', { name: /Configuration only/ }));
    const addPath = kind === 'SwarmService' ? '/platforms/platform/services' : path;
    expect(screen.getByLabelText('Page URL')).toHaveTextContent(`${addPath}/add?duplicateFrom=source`);
    await open();
    await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
    await waitFor(() =>
      expect(screen.getByLabelText('Page URL')).toHaveTextContent(`${path}/edit/source/?duplicate=data`),
    );
  });

  it.each(['row', 'bar'] as const)(
    'uses the list payload without fetching when the %s menu opens',
    async (placement) => {
      const read = vi.fn(() => HttpResponse.json(source));
      server.use(http.get('http://localhost/api/v1/*', read));
      const Group = group.duplicate;
      const renderMenu = (
        item: Omit<typeof source, 'copyEligibility'> & { copyEligibility: ResourceCopyEligibility | null },
      ) =>
        placement === 'row' ? (
          <RowActionMenu<any> actions={{ duplicate: dropdown.duplicate }} resource={item} />
        ) : (
          <Group resources={[item] as any} />
        );
      const { user, rerender } = renderCitadel(renderMenu(source));
      if (placement === 'row') {
        await user.click(screen.getByRole('button', { name: 'Open menu' }));
        screen.getByRole('menuitem', { name: 'Duplicate' }).focus();
        await user.keyboard('{ArrowRight}');
      } else await user.click(screen.getByRole('button', { name: 'Duplicate' }));
      expect(await screen.findByRole('menuitem', { name: /Configuration \+ data/ })).not.toHaveAttribute(
        'aria-disabled',
        'true',
      );
      expect(read).not.toHaveBeenCalled();

      // A refreshed list payload controls availability without a separate detail read.
      rerender(
        renderMenu({
          ...source,
          copyEligibility: {
            status: ResourceCopyEligibilityStatus.Unavailable,
            reason: 'No volumes configured.',
          },
        }),
      );
      expect(screen.getByText('No volumes configured.')).toBeVisible();
      expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).toHaveAttribute('aria-disabled', 'true');
      rerender(renderMenu({ ...source, copyEligibility: null }));
      expect(screen.getByText(/Copy availability could not be checked/)).toBeVisible();
      expect(screen.getByRole('menuitem', { name: /Configuration only/ })).not.toHaveAttribute('aria-disabled', 'true');
      expect(read).not.toHaveBeenCalled();
    },
  );

  it('disables duplication for multiple selections and read-only resources', async () => {
    const Group = group.duplicate;
    const { rerender, user } = renderCitadel(<Group resources={[source, { ...source, id: 'other' }] as any} />);
    expect(screen.getByRole('button', { name: 'Duplicate' })).toBeDisabled();
    rerender(<Group resources={[{ ...source, capabilities: { canRead: true, canWrite: false } }] as any} />);
    expect(screen.getByRole('button', { name: 'Duplicate' })).toBeDisabled();
    rerender(
      <RowActionMenu<any>
        actions={{ duplicate: dropdown.duplicate }}
        resource={{ ...source, capabilities: { canRead: true, canWrite: false } }}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Open menu' }));
    expect(screen.getByRole('menuitem', { name: 'Duplicate' })).toHaveAttribute('aria-disabled', 'true');
    await user.keyboard('{ArrowRight}');
    expect(screen.queryByRole('menuitem', { name: /Configuration only/ })).not.toBeInTheDocument();
  });

  it('keeps configuration copying available when data copying is unavailable', async () => {
    const { user } = renderCitadel(
      <RowActionMenu<any>
        actions={{ duplicate: dropdown.duplicate }}
        resource={
          {
            ...source,
            copyEligibility: { status: ResourceCopyEligibilityStatus.Unavailable, reason: 'No volumes configured.' },
          } as any
        }
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Open menu' }));
    screen.getByRole('menuitem', { name: 'Duplicate' }).focus();
    await user.keyboard('{ArrowRight}');
    expect(await screen.findByText('No volumes configured.')).toBeVisible();
    expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('menuitem', { name: /Configuration only/ })).not.toHaveAttribute('aria-disabled', 'true');
  });
});
