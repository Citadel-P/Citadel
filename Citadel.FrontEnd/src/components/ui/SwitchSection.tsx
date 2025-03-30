import { Label } from './label';
import { Switch } from './switch';

// Reusable Switch Section Component
export const SwitchSection = ({
  id,
  label,
  description,
  checked,
  onToggle,
}: {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  onToggle: () => void;
}) => (
  <div className="space-y-2 flex flex-row items-center justify-between rounded-lg border p-3 shadow-xs">
    <div className="space-y-0.5">
      <Label htmlFor={id}>{label}</Label>
      <p className="text-[0.8rem] text-muted-foreground">{description}</p>
    </div>
    <Switch id={id} checked={checked} onCheckedChange={onToggle} aria-label={label} />
  </div>
);
