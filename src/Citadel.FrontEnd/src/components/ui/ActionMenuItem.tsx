import { cn } from '@/lib/utils';
import { DropdownMenuItem } from './dropdown-menu';
import React, { JSX } from 'react';

/**
 * Props for the ActionMenuItem component.
 */
interface ActionMenuItemProps {
  /**
   * The function to call when the menu item is clicked.
   */
  onClick: () => void;
  /**
   * If true, the menu item will be disabled.
   */
  disabled?: boolean;
  /**
   * The icon to display on the menu item.
   */
  icon: React.ReactNode;
  /**
   * The text label to display on the menu item.
   */
  label: string;
  /**
   * Optional additional CSS class names to apply to the menu item.
   */
  className?: string;
}

/**
 * A reusable dropdown menu item component with an icon and a label.
 *
 * @param {ActionMenuItemProps} props The props for the component.
 * @returns {JSX.Element} The rendered dropdown menu item.
 */
export const ActionMenuItem = ({
  onClick,
  disabled,
  icon,
  label,
  className,
}: ActionMenuItemProps): JSX.Element => (
  <DropdownMenuItem
    onClick={onClick}
    disabled={disabled ?? false}
    className={cn('grow rounded-sm px-3 py-2 text-[12px] font-semibold text-foreground/70', className)}
  >
    {icon}
    <span>{label}</span>
  </DropdownMenuItem>
);