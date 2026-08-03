import { ActionRunStatus, AutomationActionView, ResourceControlState } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { AutomationActionsTable } from './table';

const createAction = (enabled: boolean): AutomationActionView =>
  ({
    id: '019fbf2c-1c80-790f-a50a-11bbec0241d3',
    name: enabled ? 'Enabled action' : 'Disabled action',
    enabled,
    scheduleEnabled: false,
    controlState: ResourceControlState.Idle,
    latestRun: {
      status: ActionRunStatus.Succeeded,
      queuedAt: '2026-08-02T00:00:00Z',
    },
    tags: [],
  }) as AutomationActionView;

describe('AutomationActionsTable', () => {
  it('shows the disabled state instead of a successful last-run status for a disabled action', () => {
    renderCitadel(<AutomationActionsTable items={[createAction(false)]} actions={{}} isLoading={false} />);

    const indicator = screen.getByRole('link', { name: 'Disabled action' }).previousElementSibling;

    expect(indicator).toHaveClass('bg-gray-500');
    expect(indicator).not.toHaveClass('bg-green-500');
  });

  it('continues to show the last-run status for an enabled action', () => {
    renderCitadel(<AutomationActionsTable items={[createAction(true)]} actions={{}} isLoading={false} />);

    const indicator = screen.getByRole('link', { name: 'Enabled action' }).previousElementSibling;

    expect(indicator).toHaveClass('bg-green-500');
  });
});
