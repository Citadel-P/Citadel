import { render, screen } from '@testing-library/react';
import { getStateIconAppearance, StateIndicator } from './state-indicator';

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

it('shows unavailable backup repositories as an error', () => {
  render(<StateIndicator variant="badge" value="Unavailable" />);
  expect(screen.getByText('Unavailable')).toHaveClass('text-red-700');
});

it.each([
  ['Info', 'bg-blue-500', 'text-blue-700'],
  ['Warning', 'bg-orange-500', 'text-orange-700'],
  ['Critical', 'bg-red-500', 'text-red-700'],
])('keeps alert severity %s filters, badges and dots consistent', (value, dot, badge) => {
  const { container, rerender } = render(<StateIndicator kind="alertSeverity" value={value} />);
  expect(container.querySelector('.rounded-full')).toHaveClass(dot);
  expect(getStateIconAppearance(value, 'alertSeverity').className).toBe(dot.replace('bg-', 'text-'));
  rerender(<StateIndicator kind="alertSeverity" value={value} variant="badge" />);
  expect(screen.getByText(value)).toHaveClass(badge);
});

it.each([
  ['Running', 'bg-green-500'],
  ['Paused', 'bg-orange-500'],
  ['Restarting', 'bg-yellow-500'],
  ['Removing', 'bg-yellow-500'],
  ['Exited', 'bg-gray-500'],
  ['Dead', 'bg-red-500'],
  ['Offline', 'bg-red-500'],
  ['Created', 'bg-blue-400'],
  ['Unknown', 'bg-gray-400'],
])('uses the same container indicator for Docker and API state %s', (state, color) => {
  const { container, rerender } = render(<StateIndicator value={state} kind="container" />);
  expect(container.querySelector('.rounded-full')).toHaveClass(color);
  rerender(<StateIndicator value={state.toLowerCase()} kind="container" />);
  expect(container.querySelector('.rounded-full')).toHaveClass(color);
  rerender(<StateIndicator value={state.toLowerCase()} kind="container" variant="badge" />);
  expect(screen.getByText(state)).toBeVisible();
});

it.each([
  ['running', 'Running', 'bg-blue-500', 'text-blue-700'],
  ['backingOff', 'Backing off', 'bg-orange-500', 'text-orange-700'],
  ['failed', 'Failed', 'bg-red-500', 'text-red-700'],
  ['unavailable', 'Unavailable', 'bg-violet-400', 'text-violet-700'],
  ['notStarted', 'Not started', 'bg-slate-500', 'text-slate-700'],
  ['stopped', 'Stopped', 'bg-gray-400', 'text-muted-foreground'],
  ['waiting', 'Waiting', 'bg-gray-400', 'text-muted-foreground'],
  ['disabled', 'Disabled', 'bg-gray-400', 'text-muted-foreground'],
])('keeps job %s badges and dots consistent', (value, label, dot, badge) => {
  const { container, rerender } = render(<StateIndicator kind="job" value={value} />);
  expect(container.querySelector('.rounded-full')).toHaveClass(dot);
  rerender(<StateIndicator kind="job" value={value} variant="badge" />);
  expect(screen.getByText(label)).toHaveClass(badge);
});
