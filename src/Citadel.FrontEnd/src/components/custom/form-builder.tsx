import { toast } from 'sonner';
import { pickDraftFields, useFormDraftKey } from '@/lib/form-drafts';
import { notifyRequestError } from '@/lib/request-error';
import React, { useMemo, useState, useCallback, useRef, memo, Ref, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Loader2, Eye, History, Save, X, ChevronRight, CircleAlert } from 'lucide-react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { MonacoDiff } from '@/lib/monaco';
import { fromNow } from '@/lib/dayjs.helper';
import { Switch } from '../ui/switch';
import { Label } from '../ui/label';
import { cn } from '@/lib/utils';
import { Textarea } from '../ui/textarea';
import { Slider } from '../ui/slider';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from '../ui/select';
import { LicenseFeatureIndicator, LicensedFeatureLabel, type LicenseEdition } from './license-feature-indicator';
import { FormFieldAccessibilityContext, useFormFieldAccessibility } from './form-field-accessibility';

type Primitive = string | number | boolean | bigint | symbol | null | undefined | Date;

// Produces: "name" | "type" | "configuration" | "configuration.pat" | ...
export type Path<T> = T extends Primitive
  ? never
  : {
      [K in keyof T & string]: T[K] extends Primitive ? K : T[K] extends Array<any> ? K : K | `${K}.${Path<T[K]>}`;
    }[keyof T & string];

export interface FieldConfig<T> {
  id?: string;
  dirtyKey?: string;
  key: Path<T>;
  label: string;
  required?: boolean;
  placeholder?: string;
  description?: React.ReactElement | string;
  requiredLicense?: LicenseEdition;
  disabled?: boolean;
  ignoreFormDisabled?: boolean;
  validate?: (value: any) => string | null;
  hideValidationMessage?: boolean;
  /** Opt in only after auditing this scalar field for credentials or other secrets. */
  persistDraft?: boolean;
  render: (value: any, set: FieldChange<T>) => React.ReactNode;
}

export interface FieldItemConfig<T> {
  kind: 'field';
  field: FieldConfig<T>;
}

export interface RowFieldConfig<T> {
  kind: 'row';
  id: string;
  fields: FieldConfig<T>[];
  gap?: string;
  className?: string;
}

export interface GroupFieldConfig<T> {
  kind: 'group';
  id: string;
  label: string;
  title?: string;
  description?: React.ReactElement | string;
  requiredLicense?: LicenseEdition;
  items: Array<FieldItemConfig<T> | RowFieldConfig<T>>;
  direction?: 'vertical' | 'horizontal';
}

export interface SectionConfig<T> {
  title?: string;
  items: SectionItemConfig<T>[];
}

export type FormSchema<T> = Record<string, SectionConfig<T>>;
export type SectionItemConfig<T> = FieldItemConfig<T> | GroupFieldConfig<T> | RowFieldConfig<T>;
export type FieldChange<T> = (partial: Partial<T> | ((prev: Partial<T>) => Partial<T>)) => void;

interface FieldShellProps {
  label: string;
  required?: boolean;
  description?: React.ReactElement | string;
  requiredLicense?: LicenseEdition;
  edited: boolean;
  error?: string | null;
  hideValidationMessage?: boolean;
  touched: boolean;
  children: React.ReactNode;
}

type DraftInfo = {
  hasDraft: boolean;
  savedAt?: string;
};

interface StoredDraft<T> {
  version?: string | number;
  update: Partial<T>;
  savedAt: string;
}

/* -------------------------------------------------------------------------- */
/*                               Schema helpers                               */
/* -------------------------------------------------------------------------- */

export function defineField<T, K extends Path<T> = Path<T>>(
  config: Omit<FieldConfig<T>, 'key'> & { key: K },
): FieldItemConfig<T> {
  return {
    kind: 'field',
    field: config,
  };
}

export function defineRowField<T>(config: {
  id: string;
  fields: Array<FieldItemConfig<T> | FieldConfig<T>>;
  gap?: string;
  className?: string;
}): RowFieldConfig<T> {
  const normalizedFields = config.fields.map((f) =>
    'kind' in f && f.kind === 'field' ? f.field : (f as FieldConfig<T>),
  );
  return {
    kind: 'row',
    id: config.id,
    fields: normalizedFields,
    gap: config.gap,
    className: config.className,
  };
}

export function defineGroupField<T>(config: {
  id: string;
  label: string;
  title?: string;
  description?: React.ReactElement | string;
  requiredLicense?: LicenseEdition;
  items: Array<FieldItemConfig<T> | RowFieldConfig<T>>;
  direction?: 'vertical' | 'horizontal';
}): GroupFieldConfig<T> {
  return {
    kind: 'group',
    id: config.id,
    label: config.label,
    title: config.title,
    description: config.description,
    requiredLicense: config.requiredLicense,
    items: config.items,
    direction: config.direction ?? 'vertical',
  };
}

export function defineSection<T>(config: SectionConfig<T>): SectionConfig<T> {
  return config;
}

/* -------------------------------------------------------------------------- */
/*                               Shell components                             */
/* -------------------------------------------------------------------------- */

function FieldShell({
  label,
  required,
  description,
  requiredLicense,
  edited,
  error,
  hideValidationMessage,
  touched,
  children,
}: FieldShellProps) {
  const showError = touched && error && !hideValidationMessage;
  const accessibilityId = React.useId();
  const descriptionId = description ? `${accessibilityId}-description` : undefined;
  const errorId = showError ? `${accessibilityId}-error` : undefined;
  const describedBy = [descriptionId, errorId].filter(Boolean).join(' ') || undefined;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <label className="block font-medium text-sm">
            {label}
            {required && <span className="text-destructive ml-1">*</span>}
          </label>
          {typeof description === 'string' ? (
            <p id={descriptionId} className="text-sm text-muted-foreground">
              {description}
            </p>
          ) : (
            description && <div id={descriptionId}>{description}</div>
          )}
        </div>
        {requiredLicense && <LicenseFeatureIndicator edition={requiredLicense} className="mt-0.5" />}
      </div>

      <div className="relative">
        {edited && <span className="absolute top-0 right-1 text-[10px] text-primary bg-background px-1">Edited</span>}
        <FormFieldAccessibilityContext.Provider
          value={{
            label,
            describedBy,
            invalid: Boolean(showError),
          }}>
          {children}
        </FormFieldAccessibilityContext.Provider>
      </div>

      {showError && (
        <p id={errorId} className="text-xs text-destructive mt-1">
          {error}
        </p>
      )}
    </div>
  );
}

// Memoized wrapper to prevent re-rendering fields when unrelated fields change.
interface SmartFieldProps<T> extends Omit<FieldShellProps, 'children'> {
  render: (value: any, set: FieldChange<T>) => React.ReactNode;
  value: any;
  onChange: FieldChange<T>;
}

function SmartFieldImpl<T>({ render, value, onChange, ...props }: SmartFieldProps<T>) {
  return <FieldShell {...props}>{render(value, onChange)}</FieldShell>;
}

const SmartField = memo(SmartFieldImpl) as typeof SmartFieldImpl;

export const FieldInput = ({
  value,
  autoFocus,
  onChange,
  onKeyDown,
  placeholder,
  type,
  min,
  max,
  step,
  disabled,
  readOnly,
  className,
  ref,
  id,
}: {
  value?: string | number;
  autoFocus?: boolean;
  onChange: (v: any) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  placeholder?: string;
  type?: string;
  min?: number | string;
  max?: number | string;
  step?: number | string;
  disabled?: boolean;
  readOnly?: boolean;
  className?: string;
  ref?: Ref<HTMLInputElement> | undefined;
  id?: string;
}) => {
  const accessibility = useFormFieldAccessibility();

  return (
    <Input
      disabled={disabled}
      readOnly={readOnly}
      type={type}
      min={min}
      max={max}
      step={step}
      ref={ref}
      autoFocus={autoFocus}
      value={value ?? ''}
      id={id}
      aria-label={accessibility?.label}
      aria-describedby={accessibility?.describedBy}
      aria-invalid={accessibility?.invalid || undefined}
      onKeyDown={onKeyDown}
      onChange={(e) => {
        const v = e.target.value;
        if (type === 'number') {
          if (v === '') {
            onChange(undefined);
          } else {
            const num = parseFloat(v);
            onChange(isNaN(num) ? v : num);
          }
        } else {
          onChange(v);
        }
      }}
      placeholder={placeholder}
      className={cn('max-w-100 max-h-9', className)}
    />
  );
};

export const FieldTextArea = ({
  value,
  onChange,
  placeholder,
  disabled,
}: {
  value?: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  disabled?: boolean;
}) => {
  const accessibility = useFormFieldAccessibility();

  return (
    <Textarea
      disabled={disabled}
      value={value ?? ''}
      aria-label={accessibility?.label}
      aria-describedby={accessibility?.describedBy}
      aria-invalid={accessibility?.invalid || undefined}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className="max-w-full focus-visible:ring-0"
    />
  );
};

export const FieldSwitch = ({
  checked,
  id,
  onChange,
  disabled,
}: {
  checked: boolean;
  id: string;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) => {
  const accessibility = useFormFieldAccessibility();

  return (
    <div className="flex items-center gap-2">
      <Switch
        checked={checked ?? false}
        id={id}
        aria-label={accessibility?.label}
        aria-describedby={accessibility?.describedBy}
        aria-invalid={accessibility?.invalid || undefined}
        onCheckedChange={onChange}
        disabled={disabled}
      />

      <Label
        htmlFor={id}
        className={cn(
          'text-sm transition-colors font-normal',
          disabled && 'opacity-60',
          checked ? 'text-green-600/80' : 'text-muted-foreground',
        )}>
        {checked ? 'On' : 'Off'}
      </Label>
    </div>
  );
};

export const FieldSelect = <TValue extends string>({
  value,
  onChange,
  options,
  disabled,
  placeholder,
  className,
}: {
  value?: TValue | null;
  onChange: (v: TValue) => void;
  options: Array<{
    value: TValue;
    label: React.ReactNode;
    disabled?: boolean;
    requiredLicense?: LicenseEdition;
  }>;
  disabled?: boolean;
  placeholder?: string;
  className?: string;
}) => {
  const accessibility = useFormFieldAccessibility();

  return (
    <Select value={value ?? undefined} onValueChange={(next) => onChange(next as TValue)} disabled={disabled}>
      <SelectTrigger
        aria-label={accessibility?.label}
        aria-describedby={accessibility?.describedBy}
        aria-invalid={accessibility?.invalid || undefined}
        className={cn('w-full max-w-100', className)}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent className="bg-background">
        {options.map((option) => (
          <SelectItem key={option.value} value={option.value} disabled={option.disabled}>
            <LicensedFeatureLabel requiredLicense={option.requiredLicense}>{option.label}</LicensedFeatureLabel>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
};

export const FieldSlider = ({
  value,
  min = 0,
  max = 100,
  step = 1,
  unit = '%',
  disabled,
  onChange,
}: {
  value?: number | null;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
  disabled?: boolean;
  onChange: (v: number) => void;
}) => {
  const current = typeof value === 'number' ? value : min;
  const accessibility = useFormFieldAccessibility();

  return (
    <div className="flex items-center gap-4 max-w-100">
      <Slider
        min={min}
        max={max}
        step={step}
        value={[current]}
        onValueChange={([v]) => onChange(v)}
        disabled={disabled}
        aria-label={accessibility?.label}
        aria-describedby={accessibility?.describedBy}
        aria-invalid={accessibility?.invalid || undefined}
        className="flex-1"
      />
      <span className="text-sm text-muted-foreground font-normal tabular-nums w-12 text-right">
        {current}
        {unit}
      </span>
    </div>
  );
};

export function PortMappingField({
  set,
  ports,
  disabled,
}: {
  set: (v: string[]) => void;
  ports: string[];
  disabled?: boolean;
}) {
  return (
    <div className="col-span-2 ">
      <div className="space-y-2 flex flex-col gap-2">
        {ports.length === 0 && <span className="text-xs text-muted-foreground">No ports exposed in this image</span>}

        {ports.map((value, idx) => {
          const hasMapping = value.includes(':');
          const [hostPort, containerPort] = hasMapping ? value.split(':') : ['', value];

          return (
            <div key={idx} className="flex flex-col sm:flex-row gap-2 items-start sm:items-center max-w-100 max-h-9">
              <div className="flex flex-1 w-full">
                <Input
                  placeholder="Host port"
                  className="rounded-r-none! focus-visible:ring-transparent"
                  type="number"
                  min="0"
                  max="65535"
                  value={hostPort}
                  disabled={disabled}
                  onChange={(e) => {
                    const newHost = e.target.value;
                    const updated = [...ports];

                    updated[idx] = newHost ? `${newHost}:${containerPort}` : containerPort;

                    set(updated);
                  }}
                />

                <span className="flex z-10 items-center justify-center w-20 shadow-xs shrink-0 bg-accent/60 border-r rounded-r-sm border-y border-border text-xs">
                  :{containerPort}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function InputGroupField({
  value,
  onChange,
  prefixPlaceholder,
  suffixPlaceholder,
  type,
  disabled,
  className,
}: {
  value?: string;
  onChange: (v: string) => void;
  prefixPlaceholder?: string;
  suffixPlaceholder?: string;
  type?: string;
  disabled?: boolean;
  className?: string;
}) {
  return (
    <div className="col-span-2 ">
      <div className="space-y-2 flex flex-col gap-2">
        <div className={cn('flex flex-col sm:flex-row gap-2 items-start sm:items-center max-w-100 max-h-9', className)}>
          <div className="flex flex-1 w-full">
            <span className="flex z-10 items-center justify-center w-20 shadow-xs shrink-0 bg-accent/60 border-l rounded-l-sm border-y border-border text-xs">
              {prefixPlaceholder}
            </span>
            <Input
              placeholder={suffixPlaceholder}
              className="rounded-l-none! focus-visible:ring-transparent"
              type={type}
              value={value}
              disabled={disabled}
              onChange={(e) => onChange(e.target.value)}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

type ItemInfo =
  | string
  | {
      label: string;
      description?: string;
      disabled?: boolean;
      requiredLicense?: LicenseEdition;
    };

export function ItemSelector({
  value,
  onChange,
  disabled,
  collection,
  className,
}: {
  value?: string;
  onChange: (v: any) => void;
  disabled?: boolean;
  collection: Record<string, ItemInfo>;
  className?: string;
}) {
  const accessibility = useFormFieldAccessibility();
  const normalized = Object.fromEntries(
    Object.entries(collection).map(([key, item]) => [key, typeof item === 'string' ? { label: item } : item]),
  );

  const finalValue = value && normalized[value] ? value : '';
  const selected = normalized[finalValue];
  return (
    <Select value={finalValue} onValueChange={onChange} disabled={disabled}>
      <SelectTrigger
        aria-label={accessibility?.label}
        aria-describedby={accessibility?.describedBy}
        aria-invalid={accessibility?.invalid || undefined}
        className={cn('w-full max-w-100', className)}>
        <SelectValue placeholder="Select a value...">
          {selected && (
            <LicensedFeatureLabel requiredLicense={selected.requiredLicense}>{selected.label}</LicensedFeatureLabel>
          )}
        </SelectValue>
      </SelectTrigger>

      <SelectContent className="bg-background">
        {Object.entries(normalized).map(([key, info]) => (
          <SelectItem key={key} value={key} disabled={info.disabled}>
            <div className="flex min-w-0 flex-1 items-center gap-2">
              <div className="flex min-w-0 flex-1 flex-col">
                <span className={info.description ? 'font-medium' : ''}>{info.label}</span>
                {info.description && <span className="text-xs text-muted-foreground">{info.description}</span>}
              </div>
              {info.requiredLicense && <LicenseFeatureIndicator edition={info.requiredLicense} />}
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/* -------------------------------------------------------------------------- */
/*                                 Form logic                                 */
/* -------------------------------------------------------------------------- */

function getValue(obj: any, path: string): any {
  return path.split('.').reduce((acc, key) => acc?.[key], obj);
}

function isObject(item: any): boolean {
  return item && typeof item === 'object' && !Array.isArray(item);
}

function deepMerge<T>(target: any, source: any): T {
  if (!isObject(target) || !isObject(source)) return (source ?? target) as T;

  if (source.$type && target.$type && source.$type !== target.$type) {
    return source as T;
  }

  const output: any = Array.isArray(target) ? [...target] : { ...target };

  for (const key of Object.keys(source)) {
    const prev = target[key];
    const next = source[key];

    if (isObject(prev) && isObject(next)) {
      output[key] = deepMerge(prev, next);
    } else {
      output[key] = next;
    }
  }

  return output as T;
}

function extractFieldMap<T>(schema: FormSchema<T>): Record<string, FieldConfig<T>> {
  const map: Record<string, FieldConfig<T>> = {};

  for (const sectionKey of Object.keys(schema)) {
    const section = schema[sectionKey];
    for (const item of section.items) {
      if (item.kind === 'field') {
        map[fieldIdentity(item.field)] = item.field;
      } else if (item.kind === 'row') {
        for (const f of item.fields) {
          map[fieldIdentity(f)] = f;
        }
      } else if (item.kind === 'group') {
        for (const sub of item.items) {
          if (sub.kind === 'field') {
            map[fieldIdentity(sub.field)] = sub.field;
          } else if (sub.kind === 'row') {
            for (const f of sub.fields) {
              map[fieldIdentity(f)] = f;
            }
          }
        }
      }
    }
  }

  return map;
}

function computeValidationState<T>(
  original: T,
  merged: T,
  fieldMap: Record<string, FieldConfig<T>>,
): {
  errors: Record<string, string | null>;
  dirty: Record<string, boolean>;
} {
  const errors: Record<string, string | null> = {};
  const dirty: Record<string, boolean> = {};

  for (const [identity, field] of Object.entries(fieldMap)) {
    const key = field.key as string;
    const dirtyKey = fieldDirtyKey(field);
    const val = getValue(merged, key);
    const currentDirtyVal = getValue(merged, dirtyKey);
    const originalDirtyVal = getValue(original, dirtyKey);

    dirty[identity] = !areValuesEqual(currentDirtyVal, originalDirtyVal);

    const isEmpty = val === undefined || val === '';
    let err: string | null = null;

    if (field.required && isEmpty) {
      err = 'Required';
    } else if (field.validate) {
      err = field.validate(val);
    }

    errors[identity] = err;
  }

  return { errors, dirty };
}

type FormNavigationItem = {
  id: string;
  label: string;
  dirty: boolean;
  error: boolean;
};

type FormNavigationSection = {
  key: string;
  title?: string;
  items: FormNavigationItem[];
};

const fieldIdentity = <T,>(field: FieldConfig<T>) => field.id ?? (field.key as string);
const fieldDirtyKey = <T,>(field: FieldConfig<T>) => field.dirtyKey ?? (field.key as string);

function buildNavigationSections<T>(
  schema: FormSchema<T>,
  dirty: Record<string, boolean>,
  errors: Record<string, string | null>,
  touched: Record<string, boolean>,
  mode: 'add' | 'edit',
): FormNavigationSection[] {
  const fieldState = (field: FieldConfig<T>) => {
    const identity = fieldIdentity(field);
    const key = field.key as string;

    return {
      dirty: mode === 'edit' && !!dirty[identity],
      error: (!!touched[identity] || !!touched[key]) && !!errors[identity],
    };
  };

  const rowState = (fields: FieldConfig<T>[]) =>
    fields.reduce(
      (state, field) => {
        const next = fieldState(field);
        return {
          dirty: state.dirty || next.dirty,
          error: state.error || next.error,
        };
      },
      { dirty: false, error: false },
    );

  const groupState = (group: GroupFieldConfig<T>) =>
    group.items.reduce(
      (state, item) => {
        const next = item.kind === 'field' ? fieldState(item.field) : rowState(item.fields);
        return {
          dirty: state.dirty || next.dirty,
          error: state.error || next.error,
        };
      },
      { dirty: false, error: false },
    );

  return Object.entries(schema).map(([sectionKey, section]) => ({
    key: sectionKey,
    title: section.title?.trim() || sectionKey.charAt(0).toUpperCase() + sectionKey.slice(1),
    items: section.items.map((item): FormNavigationItem => {
      if (item.kind === 'field') {
        const identity = fieldIdentity(item.field);
        return {
          id: identity,
          label: item.field.label,
          ...fieldState(item.field),
        };
      }

      if (item.kind === 'row') {
        return {
          id: item.id,
          label:
            item.fields
              .map((field) => field.label.trim())
              .filter(Boolean)
              .join(' / ') || item.id,
          ...rowState(item.fields),
        };
      }

      return {
        id: item.id,
        label: item.label,
        ...groupState(item),
      };
    }),
  }));
}

const FormNavigationLink = ({
  item,
  active,
  onSelect,
}: {
  item: FormNavigationItem;
  active: boolean;
  onSelect: (id: string) => void;
}) => (
  <a
    href={`#${item.id}`}
    aria-label={`${item.label}${item.error ? ', Needs attention' : item.dirty ? ', Edited' : ''}`}
    aria-current={active ? 'location' : undefined}
    className={cn(
      'relative flex min-h-(--control-height) w-full items-center gap-2 rounded-md px-3 py-[calc(var(--surface-padding)/4)] text-sm text-muted-foreground transition-colors after:pointer-events-none after:absolute after:inset-y-2 after:left-0 after:w-0.5 hover:bg-accent hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring',
      active && 'bg-primary/5 font-medium text-foreground after:bg-primary',
      item.error && 'text-destructive',
    )}
    onClick={(event) => {
      event.preventDefault();
      onSelect(item.id);
    }}>
    <span className="min-w-0 flex-1 whitespace-normal wrap-anywhere">{item.label}</span>
    {item.error ? (
      <span title="Needs attention" className="shrink-0 text-destructive">
        <CircleAlert aria-hidden="true" className="size-3.5" />
        <span className="sr-only">Needs attention</span>
      </span>
    ) : item.dirty ? (
      <span className="shrink-0 rounded-sm border border-primary/20 bg-primary/5 px-1.5 py-0.5 text-[10px] font-medium text-foreground">
        Edited
      </span>
    ) : active ? (
      <ChevronRight aria-hidden="true" className="size-3.5 shrink-0" />
    ) : null}
  </a>
);

const FormNavigationSelectItem = ({ item }: { item: FormNavigationItem }) => (
  <SelectItem
    value={item.id}
    className={cn('text-xs font-normal text-foreground/90', item.error && 'text-destructive focus:text-destructive')}>
    <span
      className={cn(
        'size-1.5 shrink-0 rounded-full bg-transparent',
        item.dirty && 'bg-primary',
        item.error && 'bg-destructive',
      )}
    />
    <span className="min-w-0 flex-1 truncate font-normal">{item.label}</span>
    {item.error ? (
      <span className="text-[10px] text-destructive">Needs attention</span>
    ) : (
      item.dirty && <span className="text-[10px] text-muted-foreground">Edited</span>
    )}
  </SelectItem>
);

/* ------------------------------ Draft helpers ----------------------------- */

function loadDraft<T>(key: string, expectedVersion?: string | number): StoredDraft<T> | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;

    const parsed = JSON.parse(raw) as StoredDraft<T>;
    if (!parsed || typeof parsed !== 'object') return null;

    if (expectedVersion !== undefined && parsed.version !== expectedVersion) {
      clearDraft(key);
      toast.info('An older browser draft was discarded because the saved fields changed.');
      return null;
    }

    if (!parsed.update || typeof parsed.update !== 'object') return null;

    return parsed;
  } catch {
    return null;
  }
}

function persistDraft<T>(key: string, version: string | number | undefined, update: Partial<T>): string | undefined {
  try {
    const savedAt = new Date().toISOString();
    const payload: StoredDraft<T> = {
      version,
      update,
      savedAt,
    };
    localStorage.setItem(key, JSON.stringify(payload));
    return savedAt;
  } catch {
    return undefined;
  }
}

function clearDraft(key: string) {
  try {
    localStorage.removeItem(key);
  } catch {
    // ignore
  }
}

/* -------------------------------------------------------------------------- */
/*                                 FormShell                                  */
/* -------------------------------------------------------------------------- */

export function FormShell<T>({
  title,
  schema,
  original,
  update,
  setUpdate,
  onSave,
  disabled,
  mode = 'edit',
  pending = false,
  draftKey: unscopedDraftKey,
  draftVersion: schemaVersion,
  onReset,
  saveLabel = 'Save',
  saveDisabled = false,
  confirmSave,
}: {
  title?: string;
  schema: FormSchema<T>;
  original: T;
  update: Partial<T>;
  setUpdate: React.Dispatch<React.SetStateAction<Partial<T>>>;
  onSave: (payload: T) => Promise<void>;
  disabled?: boolean;
  mode?: 'add' | 'edit';
  pending?: boolean;
  draftKey?: string;
  draftVersion?: string | number;
  onReset?: () => void;
  saveLabel?: string;
  saveDisabled?: boolean;
  confirmSave?: (payload: T) => Promise<boolean>;
}) {
  const draftKey = useFormDraftKey(unscopedDraftKey);
  const updateRef = React.useRef(update);
  updateRef.current = update;

  const [touched, setTouched] = useState<Record<string, boolean>>({});
  const [previewOpen, setPreviewOpen] = useState(false);
  const [optimisticOriginal, setOptimisticOriginal] = useState<T | null>(null);

  const [draftInfo, setDraftInfo] = useState<DraftInfo>({ hasDraft: false });
  const [draftLoadedBanner, setDraftLoadedBanner] = useState(false);

  const sections = Object.keys(schema);

  const fieldMap = useMemo(() => extractFieldMap(schema), [schema]);
  const draftPaths = JSON.stringify(
    Object.values(fieldMap)
      .filter((field) => field.persistDraft)
      .map((field) => field.key),
  );
  const safeDraft = useCallback(
    (value: Partial<T>) => pickDraftFields(value, JSON.parse(draftPaths) as string[]),
    [draftPaths],
  );
  // Compare only persisted fields: no credentials enter the draft or its version.
  const draftVersion = JSON.stringify([schemaVersion, safeDraft(original as Partial<T>)]);
  const lastDraftKey = useRef(draftKey);
  const effectiveOriginal = useMemo(() => optimisticOriginal ?? original, [optimisticOriginal, original]);
  const merged = useMemo(() => deepMerge<T>(effectiveOriginal, update), [effectiveOriginal, update]);

  const { errors, dirty } = useMemo(
    () => computeValidationState(effectiveOriginal, merged, fieldMap),
    [effectiveOriginal, merged, fieldMap],
  );
  const navigationSections = useMemo(
    () => buildNavigationSections(schema, dirty, errors, touched, mode),
    [schema, dirty, errors, touched, mode],
  );
  const formRef = useRef<HTMLDivElement>(null);
  const [selectedNavigationId, setSelectedNavigationId] = useState<string | undefined>(
    () => window.location.hash.slice(1) || undefined,
  );
  const selectedNavigationValue = useMemo(
    () =>
      selectedNavigationId &&
      navigationSections.some((section) => section.items.some((item) => item.id === selectedNavigationId))
        ? selectedNavigationId
        : navigationSections[0]?.items[0]?.id,
    [navigationSections, selectedNavigationId],
  );

  // Reconnect only when sections change, not on each field edit.
  const navigationTargetKey = JSON.stringify(
    navigationSections.flatMap((section) => section.items.map((item) => item.id)),
  );
  useEffect(() => {
    const ids: string[] = JSON.parse(navigationTargetKey);
    const targets = ids
      .map((id) => document.getElementById(id))
      .filter((element): element is HTMLElement => !!element && !!formRef.current?.contains(element));
    const scrollRoot = formRef.current?.closest<HTMLElement>('#main-scroll-container') ?? null;
    const scrollElement = scrollRoot ?? document.scrollingElement;
    const scrollTarget = scrollRoot ?? window;
    const visible = new Set<Element>();
    let atBottom = false;
    let frame: number | undefined;
    const updateCurrent = () => {
      // The final section may never reach the top of the viewport. At the end
      // of the scroll area it takes precedence over earlier visible sections.
      atBottom =
        !!scrollElement &&
        scrollElement.scrollTop > 0 &&
        scrollElement.scrollHeight - scrollElement.scrollTop - scrollElement.clientHeight <= 2;
      const current = atBottom ? targets.at(-1) : targets.find((target) => visible.has(target));
      if (current) setSelectedNavigationId(current.id);
    };
    const onScroll = () => {
      if (frame !== undefined) return;
      frame = requestAnimationFrame(() => {
        frame = undefined;
        const reachedBottom =
          !!scrollElement &&
          scrollElement.scrollTop > 0 &&
          scrollElement.scrollHeight - scrollElement.scrollTop - scrollElement.clientHeight <= 2;
        if (reachedBottom !== atBottom) updateCurrent();
      });
    };
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) visible.add(entry.target);
          else visible.delete(entry.target);
        }
        updateCurrent();
      },
      { root: scrollRoot, rootMargin: '-112px 0px 0px 0px', threshold: 0 },
    );
    targets.forEach((target) => observer.observe(target));
    scrollTarget.addEventListener('scroll', onScroll, { passive: true });
    return () => {
      observer.disconnect();
      scrollTarget.removeEventListener('scroll', onScroll);
      if (frame !== undefined) cancelAnimationFrame(frame);
    };
  }, [navigationTargetKey]);

  const hasChanges = useMemo(() => !areValuesEqual(effectiveOriginal, merged), [effectiveOriginal, merged]);
  const isValid = useMemo(() => Object.values(errors).every((v) => !v), [errors]);
  const canSave = hasChanges;

  const lastSavedAtLabel = draftInfo.savedAt ? fromNow(draftInfo.savedAt as any) : undefined;

  const previousOriginalRef = useRef(original);

  useEffect(() => {
    if (!optimisticOriginal) {
      previousOriginalRef.current = original;
      return;
    }

    const originalCaughtUp = areValuesEqual(original, optimisticOriginal);
    const originalChanged = !areValuesEqual(previousOriginalRef.current, original);

    if (originalCaughtUp || originalChanged) {
      setOptimisticOriginal(null);
    }

    previousOriginalRef.current = original;
  }, [original, optimisticOriginal]);

  /* ------------------------- Load draft on first mount ------------------------- */
  useEffect(() => {
    if (lastDraftKey.current !== draftKey) {
      setUpdate({});
      setOptimisticOriginal(null);
      setTouched({});
      setPreviewOpen(false);
      setDraftInfo({ hasDraft: false });
      setDraftLoadedBanner(false);
      lastDraftKey.current = draftKey;
    }
    if (!draftKey) return;
    const stored = loadDraft<Partial<T>>(draftKey, draftVersion);
    if (!stored || !stored.update) return;

    const restored = safeDraft(stored.update);
    if (!Object.keys(restored).length) {
      clearDraft(draftKey);
      return;
    }
    persistDraft(draftKey, draftVersion, restored);
    setUpdate((prev) => deepMerge<Partial<T>>(prev ?? {}, restored));
    setDraftInfo({ hasDraft: true, savedAt: stored.savedAt });
    setDraftLoadedBanner(true);
  }, [draftKey, draftVersion, setUpdate, safeDraft]);

  /* ----------------------------- Field change API ----------------------------- */
  const handleChange = useCallback(
    (key: string, partialOrUpdater: Partial<T> | ((prev: Partial<T>) => Partial<T>)) => {
      if (disabled) return;

      setTouched((prev) => {
        if (prev[key]) return prev;
        return { ...prev, [key]: true };
      });

      const current = updateRef.current ?? {};
      const resolved = typeof partialOrUpdater === 'function' ? partialOrUpdater(current) : partialOrUpdater;
      const next = deepMerge<Partial<T>>(current, resolved);

      setUpdate(next);

      if (draftKey && typeof window !== 'undefined') {
        const safeUpdate = safeDraft(next);
        if (!Object.keys(safeUpdate).length) {
          clearDraft(draftKey);
          setDraftInfo({ hasDraft: false });
          return;
        }
        const savedAt = persistDraft<Partial<T>>(draftKey, draftVersion, safeUpdate);
        if (savedAt) {
          setDraftInfo({ hasDraft: true, savedAt });
        }
      }
    },
    [disabled, setUpdate, draftKey, draftVersion, safeDraft],
  );

  // Use a Ref for handleChange to ensure the cached handlers always call the latest version
  // without needing to be re-created themselves.
  const handleChangeRef = useRef(handleChange);
  handleChangeRef.current = handleChange;

  const fieldHandlersRef = useRef<Map<string, FieldChange<T>>>(new Map());

  const getFieldHandler = useCallback((key: string) => {
    if (!fieldHandlersRef.current.has(key)) {
      fieldHandlersRef.current.set(key, (partialOrUpdater) => {
        handleChangeRef.current(key, partialOrUpdater);
      });
    }
    return fieldHandlersRef.current.get(key)!;
  }, []);

  const reset = useCallback(() => {
    if (onReset) {
      onReset();
    } else {
      setUpdate({});
    }
    setOptimisticOriginal(null);
    setTouched({});
    if (draftKey && typeof window !== 'undefined') {
      clearDraft(draftKey);
      setDraftInfo({ hasDraft: false, savedAt: undefined });
      setDraftLoadedBanner(false);
    }
  }, [onReset, setUpdate, draftKey]);

  const handleNavigationSelect = useCallback((id: string) => {
    setSelectedNavigationId(id);

    if (typeof window === 'undefined') return;

    window.history.replaceState(window.history.state, '', `${window.location.pathname}${window.location.search}#${id}`);
    document.getElementById(id)?.scrollIntoView({
      block: 'start',
      behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
    });
  }, []);

  const validateAll = useCallback(
    (value: T) => {
      const { errors: allErrors } = computeValidationState(effectiveOriginal, value, fieldMap);
      const valid = Object.values(allErrors).every((v) => !v);

      // mark all fields as touched so errors become visible
      setTouched((prev) => {
        const next: Record<string, boolean> = { ...prev };
        for (const [identity, field] of Object.entries(fieldMap)) {
          next[identity] = true;
          next[field.key as string] = true;
        }
        return next;
      });

      return valid;
    },
    [effectiveOriginal, fieldMap],
  );

  const confirm = useCallback(async () => {
    const valid = validateAll(merged);
    if (!valid) return;

    let backupDraft: StoredDraft<Partial<T>> | null = null;
    try {
      if (confirmSave && !(await confirmSave(merged as T))) return;

      if (draftKey && typeof window !== 'undefined') {
        backupDraft = loadDraft<Partial<T>>(draftKey, draftVersion);
        clearDraft(draftKey);
      }

      const savedValue = merged as T;
      await onSave(savedValue);
      setOptimisticOriginal(savedValue);
      setUpdate({});
      setTouched({});

      if (draftKey && typeof window !== 'undefined') {
        setDraftInfo({ hasDraft: false, savedAt: undefined });
        setDraftLoadedBanner(false);
      }
    } catch (error) {
      notifyRequestError(error);
      if (draftKey && typeof window !== 'undefined' && backupDraft && backupDraft.update) {
        persistDraft<Partial<T>>(draftKey, backupDraft.version, safeDraft(backupDraft.update));
      }
    }
  }, [validateAll, merged, confirmSave, onSave, draftKey, draftVersion, setUpdate, safeDraft]);

  /* -------------------------------------------------------------------------- */
  /*                                    UI                                     */
  /* -------------------------------------------------------------------------- */

  return (
    <div ref={formRef} className="flex flex-col gap-6">
      {draftKey && (
        <p className="text-xs text-muted-foreground">
          Browser drafts save only the name and description. Re-enter other unsaved settings after leaving this page.
        </p>
      )}
      {/* Draft banner (if we restored a draft) */}
      {draftLoadedBanner && (
        <div className="rounded-md border border-dashed border-primary/40 bg-primary/5 px-3 py-2 text-xs text-muted-foreground flex items-center justify-between gap-3">
          <div className="flex flex-row gap-0.5">
            <span>Restored saved fields from this browser. Re-enter credentials and other unsaved settings.</span>
            {lastSavedAtLabel && <span>(updated {lastSavedAtLabel}).</span>}
          </div>
          <div className="flex items-center gap-1">
            <Button type="button" variant="ghost" className="h-6 px-2 text-xs" onClick={reset} disabled={disabled}>
              Discard draft
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label="Dismiss"
              className="h-6 w-6 text-muted-foreground"
              onClick={() => setDraftLoadedBanner(false)}>
              <X className="w-3 h-3" />
            </Button>
          </div>
        </div>
      )}

      <div className="sticky top-0 z-20 rounded-lg border bg-card p-3 shadow-xs xl:hidden">
        <div className="flex items-center justify-end">
          <Select value={selectedNavigationValue} onValueChange={handleNavigationSelect}>
            <SelectTrigger
              aria-label="Jump to section"
              className="min-h-(--control-height) w-full bg-background text-sm">
              <SelectValue placeholder="Jump to section" />
            </SelectTrigger>
            <SelectContent className="max-h-80 bg-background">
              {navigationSections.map((section, sectionIndex) => (
                <React.Fragment key={section.key}>
                  {sectionIndex > 0 && <SelectSeparator />}
                  <SelectGroup>
                    {section.title ? (
                      <SelectLabel className="text-[10px] uppercase text-muted-foreground">{section.title}</SelectLabel>
                    ) : (
                      sectionIndex === 0 &&
                      title && (
                        <SelectLabel className="text-[10px] uppercase text-muted-foreground">{title}</SelectLabel>
                      )
                    )}
                    {section.items.map((item) => (
                      <FormNavigationSelectItem key={`${section.key}:${item.id}`} item={item} />
                    ))}
                  </SelectGroup>
                </React.Fragment>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      <div className="flex min-w-0 items-start gap-(--section-gap)">
        {/* Sidebar (xl and up) */}
        <aside className="sticky top-17 hidden shrink-0 xl:block">
          <div className="flex max-h-[calc(100dvh-11rem)] min-h-0 w-60 flex-col overflow-hidden rounded-lg border bg-card shadow-xs">
            <nav
              aria-label={`${title ?? 'Form'} sections`}
              className="min-h-0 flex-1 space-y-(--section-gap) overflow-y-auto overscroll-contain p-[calc(var(--surface-padding)/2)]">
              {navigationSections.map((section) => {
                return (
                  <div key={section.key} className="flex flex-col gap-0.5">
                    {section.title && (
                      <p className="px-3 pb-1.5 pt-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                        {section.title}
                      </p>
                    )}

                    {section.items.map((item) => {
                      return (
                        <FormNavigationLink
                          key={`${section.key}:${item.id}`}
                          item={item}
                          active={selectedNavigationValue === item.id}
                          onSelect={handleNavigationSelect}
                        />
                      );
                    })}
                  </div>
                );
              })}
            </nav>

            {hasChanges && (
              <div
                data-slot="form-sidebar-actions"
                className="flex shrink-0 flex-col items-center gap-2 border-t bg-muted/10 p-[calc(var(--surface-padding)/2)]">
                {mode === 'edit' && (
                  <>
                    <Button
                      className="w-full text-xs"
                      variant="outline"
                      onClick={reset}
                      disabled={disabled || !hasChanges}>
                      <History className="w-3 h-3" /> Reset
                    </Button>

                    <Button
                      className="w-full text-xs"
                      variant="outline"
                      disabled={!hasChanges}
                      onClick={() => setPreviewOpen(true)}>
                      <Eye className="w-3 h-3" /> Preview Changes
                    </Button>
                  </>
                )}
                <Button
                  className="w-full text-xs"
                  disabled={disabled || saveDisabled || !isValid || !canSave || pending}
                  onClick={confirm}>
                  {pending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-3 h-3" />} {saveLabel}
                </Button>
              </div>
            )}
          </div>
        </aside>

        {/* Main */}
        <main className="min-w-0 flex-1 flex flex-col gap-4">
          {sections.map((sectionKey) => {
            const section = schema[sectionKey];

            return (
              <div key={sectionKey} className="flex min-w-0 flex-col gap-5 pb-4 last:pb-0">
                {section.title && <h2 className="text-base font-semibold tracking-tight">{section.title}</h2>}

                {section.items.map((item) => {
                  if (item.kind === 'field') {
                    const f = item.field;
                    const key = f.key as string;
                    const identity = fieldIdentity(f);
                    const value = getValue(merged, key);
                    const error = errors[identity];
                    const edited = mode === 'edit' && dirty[identity];
                    const fieldDisabled = f.ignoreFormDisabled ? !!f.disabled : !!disabled || !!f.disabled;

                    return (
                      <fieldset
                        id={identity}
                        key={identity}
                        disabled={fieldDisabled}
                        className={cn(
                          'relative min-w-0 border bg-card rounded-lg p-(--surface-padding) scroll-mt-22 xl:scroll-mt-20 shadow-xs',
                          fieldDisabled && 'pointer-events-none opacity-60',
                        )}>
                        <SmartField
                          render={f.render}
                          value={value}
                          onChange={getFieldHandler(key)}
                          label={f.label}
                          required={f.required}
                          description={f.description}
                          requiredLicense={f.requiredLicense}
                          edited={!!edited}
                          error={error}
                          hideValidationMessage={f.hideValidationMessage}
                          touched={!!touched[identity] || !!touched[key]}
                        />
                      </fieldset>
                    );
                  }

                  if (item.kind === 'row') {
                    // Row at section level
                    return (
                      <section
                        id={item.id}
                        key={item.id}
                        className="relative min-w-0 border bg-card rounded-lg p-(--surface-padding) scroll-mt-22 xl:scroll-mt-20">
                        <div className={cn('flex flex-row w-full', item.gap ?? 'gap-4', item.className)}>
                          {item.fields.map((f) => {
                            const key = f.key as string;
                            const identity = fieldIdentity(f);
                            const value = getValue(merged, key);
                            const error = errors[identity];
                            const edited = mode === 'edit' && dirty[identity];
                            const fieldDisabled = f.ignoreFormDisabled ? !!f.disabled : !!disabled || !!f.disabled;

                            return (
                              <fieldset
                                key={identity}
                                disabled={fieldDisabled}
                                className={cn(
                                  'relative pb-0 last:pb-0 flex-1',
                                  fieldDisabled && 'pointer-events-none opacity-60',
                                )}>
                                <SmartField
                                  render={f.render}
                                  value={value}
                                  onChange={getFieldHandler(key)}
                                  label={f.label}
                                  required={f.required}
                                  description={f.description}
                                  requiredLicense={f.requiredLicense}
                                  edited={!!edited}
                                  error={error}
                                  hideValidationMessage={f.hideValidationMessage}
                                  touched={!!touched[identity] || !!touched[key]}
                                />
                              </fieldset>
                            );
                          })}
                        </div>
                      </section>
                    );
                  }

                  // Group
                  const group = item;
                  return (
                    <section
                      id={group.id}
                      key={group.id}
                      className={`relative min-w-0 rounded-lg border border-border bg-card p-(--surface-padding) shadow-xs ${group.direction === 'horizontal' ? 'flex-row' : 'flex-col'} scroll-mt-22 xl:scroll-mt-20`}>
                      <div className="flex flex-col gap-4 w-full">
                        {(group.title || group.label || group.description || group.requiredLicense) && (
                          <div className="-mx-(--surface-padding) -mt-(--surface-padding) flex items-start justify-between gap-3 rounded-t-lg border-b bg-muted/15 px-(--surface-padding) py-3">
                            <div className="flex min-w-0 flex-col gap-1.5">
                              {(group.title || group.label) && (
                                <h3 className="text-sm font-semibold tracking-tight text-foreground/90">
                                  {group.title || group.label}
                                </h3>
                              )}
                              {typeof group.description === 'string' ? (
                                <p className="max-w-full text-sm leading-6 text-muted-foreground">
                                  {group.description}
                                </p>
                              ) : (
                                group.description
                              )}
                            </div>
                            {group.requiredLicense && (
                              <LicenseFeatureIndicator edition={group.requiredLicense} className="mt-0.5" />
                            )}
                          </div>
                        )}
                        {group.items.map((sub) => {
                          if (sub.kind === 'field') {
                            const f = sub.field;
                            const key = f.key as string;
                            const identity = fieldIdentity(f);
                            const value = getValue(merged, key);
                            const error = errors[identity];
                            const edited = mode === 'edit' && dirty[identity];
                            const fieldDisabled = f.ignoreFormDisabled ? !!f.disabled : !!disabled || !!f.disabled;

                            return (
                              <fieldset
                                key={identity}
                                disabled={fieldDisabled}
                                className={cn(
                                  `relative min-w-0 pb-6 last:pb-0 ${group.direction === 'horizontal' ? 'flex-1' : 'block border-b last:border-b-0'}`,
                                  fieldDisabled && 'pointer-events-none opacity-60',
                                )}>
                                <SmartField
                                  render={f.render}
                                  value={value}
                                  onChange={getFieldHandler(key)}
                                  label={f.label}
                                  required={f.required}
                                  description={f.description}
                                  requiredLicense={f.requiredLicense}
                                  edited={!!edited}
                                  error={error}
                                  hideValidationMessage={f.hideValidationMessage}
                                  touched={!!touched[identity] || !!touched[key]}
                                />
                              </fieldset>
                            );
                          }

                          const row = sub;
                          return (
                            <div
                              key={row.id}
                              id={row.id}
                              className={`w-full ${group.direction === 'horizontal' ? 'flex-1' : ''} rounded-md scroll-mt-22 xl:scroll-mt-20`}>
                              <div
                                className={cn('flex flex-col sm:flex-row w-full', row.gap ?? 'gap-4', row.className)}>
                                {row.fields.map((f) => {
                                  const key = f.key as string;
                                  const identity = fieldIdentity(f);
                                  const value = getValue(merged, key);
                                  const error = errors[identity];
                                  const edited = mode === 'edit' && dirty[identity];
                                  const fieldDisabled = f.ignoreFormDisabled
                                    ? !!f.disabled
                                    : !!disabled || !!f.disabled;
                                  return (
                                    <fieldset
                                      key={identity}
                                      disabled={fieldDisabled}
                                      className={cn(
                                        'pb-1 last:pb-1 last:flex-1 scroll-mt-22 xl:scroll-mt-20',
                                        fieldDisabled && 'pointer-events-none opacity-60',
                                      )}>
                                      <SmartField
                                        render={f.render}
                                        value={value}
                                        onChange={getFieldHandler(key)}
                                        label={f.label}
                                        required={f.required}
                                        description={f.description}
                                        requiredLicense={f.requiredLicense}
                                        edited={!!edited}
                                        error={error}
                                        hideValidationMessage={f.hideValidationMessage}
                                        touched={!!touched[identity] || !!touched[key]}
                                      />
                                    </fieldset>
                                  );
                                })}
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </section>
                  );
                })}
              </div>
            );
          })}
        </main>
      </div>

      {/* Bottom action bar for small screens */}
      <div className="xl:hidden sticky bottom-0 z-20 rounded-lg border bg-card p-3 shadow-sm">
        <div className="flex flex-wrap justify-end gap-2">
          {mode === 'edit' && (
            <>
              <Button variant="outline" size="sm" onClick={reset} disabled={disabled || !hasChanges}>
                <History className="w-4 h-4 mr-1" />
                Reset
              </Button>

              <Button variant="outline" size="sm" disabled={!hasChanges} onClick={() => setPreviewOpen(true)}>
                <Eye className="w-3 h-3" /> Preview Changes
              </Button>
            </>
          )}
          <Button size="sm" onClick={confirm} disabled={disabled || saveDisabled || !isValid || !canSave || pending}>
            {pending ? <Loader2 className="w-4 h-4 animate-spin mr-1" /> : <Save className="w-3 h-3 mr-1" />}{' '}
            {saveLabel}
          </Button>
        </div>
      </div>

      {/* Preview Modal */}
      <Dialog open={previewOpen} onOpenChange={setPreviewOpen}>
        <DialogContent
          aria-describedby={undefined}
          onOpenAutoFocus={(e) => {
            e.preventDefault();
          }}
          className="w-full max-w-275 sm:max-w-275">
          <DialogHeader>
            <DialogTitle>Configuration changes</DialogTitle>
          </DialogHeader>

          <MonacoDiff original={effectiveOriginal} modified={merged} format="yaml" />
        </DialogContent>
      </Dialog>
    </div>
  );
}
function areValuesEqual(a: any, b: any): boolean {
  if (a === b) return true;
  const isEmptyA =
    a === undefined ||
    a === null ||
    (typeof a === 'string' && a.trim().length === 0) ||
    (Array.isArray(a) && a.length === 0) ||
    (isObject(a) && Object.keys(a).length === 0);
  const isEmptyB =
    b === undefined ||
    b === null ||
    (typeof b === 'string' && b.trim().length === 0) ||
    (Array.isArray(b) && b.length === 0) ||
    (isObject(b) && Object.keys(b).length === 0);
  if (isEmptyA && isEmptyB) return true;

  try {
    return JSON.stringify(a) === JSON.stringify(b);
  } catch (e) {
    return false;
  }
}
