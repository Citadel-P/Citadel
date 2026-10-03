import { Palette, RotateCcw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from '@/components/ui/sheet';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { useAppearance } from '@/lib/appearance/appearance-context';
import {
  MODE_OPTIONS,
  THEME_COLORS,
  FONT_OPTIONS,
  RADIUS_OPTIONS,
  CONTENT_LAYOUT_OPTIONS,
  DENSITY_OPTIONS,
} from '@/lib/appearance/appearance-config';
import type { AppearancePreferences } from '@/lib/appearance/appearance-types';

export function AppearanceCustomizer({ showLabel = false }: { showLabel?: boolean }) {
  const { preferences, setMode, setColor, setFont, setRadius, setContentLayout, setDensity, resetAppearance } =
    useAppearance();
  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button
          variant={showLabel ? 'outline' : 'ghost'}
          size={showLabel ? 'sm' : 'icon-sm'}
          aria-label="Customize appearance">
          <Palette className="size-4" />
          {showLabel && 'Customize appearance'}
        </Button>
      </SheetTrigger>
      <SheetContent className="w-full overflow-y-auto sm:max-w-sm">
        <SheetHeader>
          <SheetTitle>Appearance</SheetTitle>
          <SheetDescription>Make Citadel your workspace. Changes apply immediately.</SheetDescription>
        </SheetHeader>
        <div className="flex flex-col gap-6 px-4 pb-6">
          <Choices label="Mode" value={preferences.mode} options={MODE_OPTIONS} onChange={setMode} />
          <fieldset className="space-y-3">
            <legend className="text-sm font-medium">Accent color</legend>
            <RadioGroup
              aria-label="Accent color"
              value={preferences.color}
              onValueChange={(value) => setColor(value as AppearancePreferences['color'])}
              className="grid grid-cols-4 gap-2">
              {THEME_COLORS.map((option) => (
                <label
                  key={option.value}
                  className="flex cursor-pointer flex-col items-center gap-2 rounded-md border p-2 has-[[data-state=checked]]:border-primary has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring">
                  <RadioGroupItem
                    value={option.value}
                    aria-label={option.label}
                    className="size-6 border-0 text-white"
                    style={{ backgroundColor: option.swatch, color: option.foreground }}
                  />
                  <span className="text-xs">{option.label}</span>
                </label>
              ))}
            </RadioGroup>
          </fieldset>
          <div className="space-y-3">
            <label htmlFor="appearance-font" className="text-sm font-medium">
              Interface font
            </label>
            <Select value={preferences.font} onValueChange={(value) => setFont(value as AppearancePreferences['font'])}>
              <SelectTrigger id="appearance-font" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {FONT_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    <span style={{ fontFamily: option.family }}>{option.label}</span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <Choices label="Corner radius" value={preferences.radius} options={RADIUS_OPTIONS} onChange={setRadius} />
          <Choices
            label="Content layout"
            value={preferences.contentLayout}
            options={CONTENT_LAYOUT_OPTIONS}
            onChange={setContentLayout}
          />
          <Choices label="Density" value={preferences.density} options={DENSITY_OPTIONS} onChange={setDensity} />
          <div className="rounded-lg border bg-muted/30 p-(--surface-padding)">
            <p className="font-medium">A workspace that fits</p>
            <p className="mt-1 text-xs text-muted-foreground">
              Operational status colors stay consistent across every accent.
            </p>
          </div>
          <Button variant="outline" onClick={resetAppearance}>
            <RotateCcw className="size-4" />
            Reset to defaults
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  );
}
function Choices<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <fieldset className="space-y-3">
      <legend className="text-sm font-medium">{label}</legend>
      <RadioGroup
        aria-label={label}
        value={value}
        onValueChange={(next) => onChange(next as T)}
        className="flex gap-1 rounded-lg border bg-muted/30 p-1">
        {options.map((option) => (
          <label
            key={option.value}
            className="relative flex min-h-(--control-height) flex-1 cursor-pointer items-center justify-center rounded-md px-2 text-xs text-muted-foreground has-[[data-state=checked]]:bg-background has-[[data-state=checked]]:text-foreground has-[[data-state=checked]]:shadow-xs has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring">
            <RadioGroupItem
              value={option.value}
              aria-label={option.label}
              className="absolute inset-0 size-full border-0 opacity-0"
            />
            {option.label}
          </label>
        ))}
      </RadioGroup>
    </fieldset>
  );
}
