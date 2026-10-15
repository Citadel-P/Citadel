import { screen, within } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { StepIndicator } from './step-indicator';

const steps = [
  { id: 'configure', title: 'Configure', description: 'Name and target' },
  { id: 'data', title: 'Select data', description: 'Backups and volumes' },
  { id: 'review', title: 'Review', description: 'Confirm the copy' },
];

it('marks the current step and only allows returning to completed steps', async () => {
  const onStepChange = vi.fn();
  const { user } = renderCitadel(<StepIndicator steps={steps} currentStep="data" onStepChange={onStepChange} />);
  const items = within(screen.getByRole('navigation', { name: 'Steps' })).getAllByRole('listitem');
  expect(items[1]).toHaveAttribute('aria-current', 'step');
  expect(within(items[2]).queryByRole('button')).not.toBeInTheDocument();
  await user.tab();
  expect(screen.getByRole('button', { name: 'Back to Configure' })).toHaveFocus();
  await user.keyboard('{Enter}');
  expect(onStepChange).toHaveBeenCalledExactlyOnceWith('configure');
});

it('locks step navigation while an operation is pending', async () => {
  const onStepChange = vi.fn();
  const { user } = renderCitadel(
    <StepIndicator steps={steps} currentStep="review" onStepChange={onStepChange} disabled />,
  );
  const back = screen.getByRole('button', { name: 'Back to Configure' });
  expect(back).toBeDisabled();
  await user.click(back);
  expect(onStepChange).not.toHaveBeenCalled();
});
