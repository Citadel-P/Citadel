import { ShieldCheck } from 'lucide-react';

export function SystemBadge({ description, ariaLabel }: { description: string; ariaLabel?: string }) {
  return (
    <div title={description} aria-label={ariaLabel ?? description} className=" text-primary">
      <ShieldCheck className="size-3.5" />
    </div>
  );
}
