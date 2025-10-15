import { Label } from '../ui/label';
import { Switch } from '../ui/switch';

interface SwitchConfig {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  onToggle: () => void;
}

export const SwitchList = ({ switches }: { switches: SwitchConfig[] }) => (
  <div className="grid gap-4 py-4">
    {switches.map((s) => (
      <SwitchSection
        key={s.id}
        id={s.id}
        label={s.label}
        description={s.description}
        checked={s.checked}
        onToggle={s.onToggle}
      />
    ))}
  </div>
);

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
