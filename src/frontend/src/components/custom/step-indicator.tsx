import { Check } from 'lucide-react';
import { cn } from '@/lib/utils';

export type WizardStep = { id: string; title: string; description?: string };

/** Displays progress; only completed steps can be revisited without validation. */
export function StepIndicator({
  steps,
  currentStep,
  onStepChange,
  disabled = false,
  label = 'Steps',
}: {
  steps: readonly WizardStep[];
  currentStep: string;
  onStepChange?: (id: string) => void;
  disabled?: boolean;
  label?: string;
}) {
  const currentIndex = steps.findIndex((step) => step.id === currentStep);

  return (
    <nav aria-label={label}>
      <ol className="flex gap-3 sm:gap-5">
        {steps.map((step, index) => {
          const completed = index < currentIndex;
          const active = index === currentIndex;
          const content = (
            <>
              <span
                aria-hidden="true"
                className={cn(
                  'flex size-7 shrink-0 items-center justify-center rounded-full border text-xs font-semibold transition-colors motion-reduce:transition-none sm:size-8',
                  active
                    ? 'border-primary bg-primary text-primary-foreground'
                    : completed
                      ? 'border-primary/25 bg-primary/10 text-primary'
                      : 'border-border bg-background text-muted-foreground',
                )}>
                {completed ? <Check className="size-3.5" /> : index + 1}
              </span>
              <span className="min-w-0">
                <span
                  className={cn(
                    'block text-xs font-medium sm:text-sm',
                    active ? 'text-foreground' : 'text-muted-foreground',
                  )}>
                  {step.title}
                </span>
                {step.description && (
                  <span className="mt-0.5 hidden text-xs text-muted-foreground lg:block">{step.description}</span>
                )}
                <span className="sr-only">
                  {completed ? ' — completed' : active ? ' — current step' : ' — upcoming'}
                </span>
              </span>
            </>
          );

          return (
            <li
              key={step.id}
              aria-current={active ? 'step' : undefined}
              className="flex min-w-0 flex-1 items-center gap-3 last:flex-initial sm:gap-5">
              {completed && onStepChange ? (
                <button
                  type="button"
                  disabled={disabled}
                  onClick={() => onStepChange(step.id)}
                  aria-label={`Back to ${step.title}`}
                  className="flex min-w-0 flex-col items-start gap-2 rounded-md text-left outline-none transition-opacity hover:opacity-75 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-4 disabled:pointer-events-none disabled:opacity-50 motion-reduce:transition-none sm:flex-row sm:items-center sm:gap-3">
                  {content}
                </button>
              ) : (
                <span className="flex min-w-0 flex-col items-start gap-2 sm:flex-row sm:items-center sm:gap-3">
                  {content}
                </span>
              )}
              {index < steps.length - 1 && (
                <span
                  aria-hidden="true"
                  className={cn('hidden h-px min-w-3 flex-1 sm:block', completed ? 'bg-primary/30' : 'bg-border')}
                />
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
