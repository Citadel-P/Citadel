import { render, screen } from '@testing-library/react';
import { StateIndicator } from './state-indicator';

it('keeps the default table indicator as a dot', () => {
  const { container } = render(<StateIndicator value="Healthy" />);
  expect(container.querySelector('[data-slot="badge"]')).not.toBeInTheDocument();
  expect(container.querySelector('.rounded-full')).toBeInTheDocument();
});

it('uses readable labels for enabled and usage badges', () => {
  const { rerender } = render(<StateIndicator variant="badge" value={true} enableLabel />);
  expect(screen.getByText('Enabled')).toHaveAttribute('data-slot', 'badge');
  rerender(<StateIndicator variant="badge" value={false} enableLabel />);
  expect(screen.getByText('Disabled')).toBeVisible();
  rerender(<StateIndicator variant="badge" value={true} />);
  expect(screen.getByText('In use')).toBeVisible();
  rerender(<StateIndicator variant="badge" value={false} />);
  expect(screen.getByText('Unused')).toBeVisible();
});

it('keeps resource-specific badge meanings and diagnostic details', () => {
  const { rerender } = render(<StateIndicator variant="badge" value="Running" kind="buildRun" />);
  expect(screen.getByText('Running')).toHaveClass('text-blue-700');
  rerender(<StateIndicator variant="badge" value="Running" kind="swarmTask" />);
  expect(screen.getByText('Running')).toHaveClass('text-green-700');
  rerender(<StateIndicator variant="badge" value="ready:drain" kind="swarmNode" />);
  expect(screen.getByText('Ready · Drained')).toHaveClass('text-muted-foreground');
  rerender(
    <StateIndicator variant="badge" value="Invalid" kind="buildAgentPoolValidation" tooltip="Credentials rejected" />,
  );
  expect(screen.getByText('Invalid')).toHaveAttribute('title', 'Credentials rejected');
});
