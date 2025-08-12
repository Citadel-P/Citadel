import { cn } from '@/lib/utils';
import React from 'react';

/**
 * Props for the ActionBarButton component.
 */
interface ActionBarButtonProps {
  /**
   * The function to call when the button is clicked.
   */
  onClick: () => void;
  /**
   * If true, the button will be disabled.
   */
  disabled: boolean;
  /**
   * The icon component to display on the button.
   */
  icon: React.ComponentType<{ className?: string }>;
  /**
   * The text label to display on the button.
   */
  label: string;
  /**
   * Optional additional CSS class names to apply to the button.
   */
  className?: string;
  /**
   * Optional aria-label for accessibility.
   */
  ariaLabel?: string;
}

/**
 * A reusable button component designed for use in an action bar.
 * It includes an icon, a label, and styling for enabled/disabled states.
 *
 * @param {ActionBarButtonProps} props The props for the component.
 * @returns {JSX.Element} The rendered button component.
 */
export const ActionBarButton = ({
  onClick,
  disabled,
  icon: Icon,
  label,
  className = '',
  ariaLabel,
}: ActionBarButtonProps): JSX.Element => (
  <button
    type="button"
    onClick={onClick}
    disabled={disabled}
    aria-label={ariaLabel}
    className={cn(
      'inline-flex items-center border border-border px-2 py-2 text-xs font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 disabled:cursor-not-allowed disabled:opacity-60',
      className,
    )}
  >
    <Icon className="mr-1 h-3 w-3" />
    {label}
  </button>
);