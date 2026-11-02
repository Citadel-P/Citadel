import './password-feedback.css';
import { cn } from '@/lib/utils';
import { estimatePasswordStrength } from '@/lib/password-strength';

const strengths = [
  { label: 'Weak', text: 'text-destructive' },
  { label: 'Fair', text: 'text-warning' },
  { label: 'Strong', text: 'text-success' },
];

export function PasswordFeedback({
  password,
  minimumLength,
  maximumLength,
  id,
}: {
  password: string;
  minimumLength: number;
  maximumLength: number;
  id?: string;
}) {
  const length = [...password].length;
  const hasPassword = length > 0;
  const tooLong = length > maximumLength;

  const score = hasPassword && !tooLong ? estimatePasswordStrength(password) : null;
  const level = score === null ? 0 : score < 2 ? 1 : score < 4 ? 2 : 3;
  const strength = level > 0 ? strengths[level - 1] : null;

  return (
    <div id={id} className="space-y-1.5 text-[11px] text-muted-foreground">
      <div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
        <span
          className={cn(
            'motion-safe:transition-colors motion-safe:duration-300',
            strength && 'font-medium',
            strength?.text,
          )}>
          {!hasPassword
            ? `Use ${minimumLength} to ${maximumLength} characters.`
            : tooLong
              ? `Maximum ${maximumLength} characters`
              : strength?.label}
        </span>
        <span className={cn('text-[11px] tabular-nums', tooLong && 'text-destructive')}>
          {length} {length === 1 ? 'character' : 'characters'}
          {hasPassword && ` · min ${minimumLength}`}
        </span>
      </div>
      <div
        role={strength ? 'meter' : undefined}
        aria-label={strength ? 'Estimated password strength' : undefined}
        aria-valuemin={strength ? 0 : undefined}
        aria-valuemax={strength ? 3 : undefined}
        aria-valuenow={strength ? level : undefined}
        aria-valuetext={strength?.label}
        aria-hidden={!hasPassword}
        className={cn(
          'flex gap-1 motion-safe:transition-opacity motion-safe:duration-200',
          !hasPassword && 'opacity-0',
        )}>
        {strengths.map((item, index) => (
          <span
            key={item.label}
            aria-hidden="true"
            data-filled={Boolean(strength && index < level)}
            className="password-meter-segment relative h-1.5 flex-1 rounded-full bg-muted">
            <span
              data-tip={Boolean(strength && index === level - 1)}
              className={cn(
                'password-meter-fill absolute inset-0 origin-left rounded-full motion-safe:transition-[transform,color] motion-safe:duration-300 motion-safe:ease-out',
                strength?.text,
              )}
              style={{
                transform: `scaleX(${strength && index < level ? 1 : 0})`,
              }}>
              <span className="password-meter-sparks absolute inset-0 overflow-hidden rounded-full" />
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}
