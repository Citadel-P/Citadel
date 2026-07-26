import React, { useMemo, useState, useCallback, useRef, memo, Ref, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Loader2, Eye, History, Save, X } from 'lucide-react';
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

export function defineField<T, K extends Path<T>>(
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
        {ports.length === 0 && <span className="text-xs text-muted">No ports exposed in this image</span>}

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
    title: section.title,
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
          label: item.id,
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
  variant,
  onSelect,
}: {
  item: FormNavigationItem;
  variant: 'sidebar' | 'compact';
  onSelect: (id: string) => void;
}) => (
  <Button
    asChild
    variant={variant === 'sidebar' ? 'secondary' : 'outline'}
    size="sm"
    className={cn(
      'text-xs font-normal text-foreground/90',
      variant === 'sidebar' ? 'w-full justify-end bg-accent/60' : 'h-8 shrink-0 rounded-sm px-2.5',
      item.error && 'border-destructive/50 bg-destructive/10 text-red-700 hover:bg-destructive/15',
    )}>
    <a
      href={`#${item.id}`}
      title={item.label}
      onClick={(event) => {
        event.preventDefault();
        onSelect(item.id);
      }}>
      {item.dirty && <span className="mr-1 text-[10px] text-destructive">*</span>}
      <span className="truncate">{item.label}</span>
    </a>
  </Button>
);

const FormNavigationSelectItem = ({ item }: { item: FormNavigationItem }) => (
  <SelectItem
    value={item.id}
    className={cn(
      'text-xs font-normal text-foreground/90',
      item.error && 'text-destructive focus:text-destructive',
    )}>
    <span
      className={cn(
        'size-1.5 shrink-0 rounded-full bg-transparent',
        item.dirty && 'bg-primary',
        item.error && 'bg-destructive',
      )}
    />
    <span className="min-w-0 flex-1 truncate font-normal">{item.label}</span>
    {item.dirty && <span className="text-[10px] text-muted-foreground">Edited</span>}
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
  draftKey,
  draftVersion,
  onReset,
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
}) {
  const updateRef = React.useRef(update);
  updateRef.current = update;

  const [touched, setTouched] = useState<Record<string, boolean>>({});
  const [previewOpen, setPreviewOpen] = useState(false);
  const [optimisticOriginal, setOptimisticOriginal] = useState<T | null>(null);

  const [draftInfo, setDraftInfo] = useState<DraftInfo>({ hasDraft: false });
  const [draftLoadedBanner, setDraftLoadedBanner] = useState(false);

  const sections = Object.keys(schema);

  const fieldMap = useMemo(() => extractFieldMap(schema), [schema]);
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
  const [selectedNavigationId, setSelectedNavigationId] = useState<string | undefined>();
  const selectedNavigationValue = useMemo(
    () =>
      selectedNavigationId &&
      navigationSections.some((section) => section.items.some((item) => item.id === selectedNavigationId))
        ? selectedNavigationId
        : undefined,
    [navigationSections, selectedNavigationId],
  );

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
    if (!draftKey) return;
    if (typeof window === 'undefined') return;

    const stored = loadDraft<Partial<T>>(draftKey, draftVersion);
    if (!stored || !stored.update) return;

    setUpdate((prev) => deepMerge<Partial<T>>(prev ?? {}, stored.update));
    setDraftInfo({ hasDraft: true, savedAt: stored.savedAt });
    setDraftLoadedBanner(true);
  }, [draftKey, draftVersion, setUpdate]);

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
        const savedAt = persistDraft<Partial<T>>(draftKey, draftVersion, next);
        if (savedAt) {
          setDraftInfo({ hasDraft: true, savedAt });
        }
      }
    },
    [disabled, setUpdate, draftKey, draftVersion],
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

    window.history.replaceState(null, '', `${window.location.pathname}${window.location.search}#${id}`);
    document.getElementById(id)?.scrollIntoView({ block: 'start', behavior: 'smooth' });
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
    if (draftKey && typeof window !== 'undefined') {
      backupDraft = loadDraft<Partial<T>>(draftKey, draftVersion);
      clearDraft(draftKey);
    }

    try {
      const savedValue = merged as T;
      await onSave(savedValue);
      setOptimisticOriginal(savedValue);
      setUpdate({});
      setTouched({});

      if (draftKey && typeof window !== 'undefined') {
        setDraftInfo({ hasDraft: false, savedAt: undefined });
        setDraftLoadedBanner(false);
      }
    } catch {
      if (draftKey && typeof window !== 'undefined' && backupDraft && backupDraft.update) {
        persistDraft<Partial<T>>(draftKey, backupDraft.version, backupDraft.update);
      }
    }
  }, [validateAll, merged, onSave, draftKey, draftVersion, setUpdate]);

  /* -------------------------------------------------------------------------- */
  /*                                    UI                                     */
  /* -------------------------------------------------------------------------- */

  return (
    <div className="flex flex-col gap-6">
      {/* Draft banner (if we restored a draft) */}
      {draftLoadedBanner && (
        <div className="rounded-md border border-dashed border-primary/40 bg-primary/5 px-3 py-2 text-xs text-muted-foreground flex items-center justify-between gap-3">
          <div className="flex flex-row gap-0.5">
            <span>Restored an unsaved draft from this browser</span>
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

      <div className="sticky top-0 z-20 -mx-1 border-b bg-background/95 px-1 py-2 backdrop-blur supports-[backdrop-filter]:bg-background/80 xl:hidden">
        <div className="flex items-center justify-end">
          <Select value={selectedNavigationValue} onValueChange={handleNavigationSelect}>
            <SelectTrigger className="h-8 w-full mb-3 rounded-sm bg-background text-xs shadow-xs">
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

      <div className="flex gap-6">
        {/* Sidebar (xl and up) */}
        <aside className="hidden xl:block relative pr-6 border-r">
          <div className="sticky top-26 hidden xl:flex flex-col gap-8 w-35 h-fit pb-24">
            {title && <p className="text-sm font-semibold text-muted-foreground mb-2">{title}</p>}

            {navigationSections.map((section) => {
              return (
                <div key={section.key} className="flex flex-col gap-2">
                  {section.title && (
                    <p className="uppercase text-xs mb-1 text-muted-foreground text-right">{section.title}</p>
                  )}

                  {section.items.map((item) => {
                    return (
                      <FormNavigationLink
                        key={`${section.key}:${item.id}`}
                        item={item}
                        variant="sidebar"
                        onSelect={handleNavigationSelect}
                      />
                    );
                  })}
                </div>
              );
            })}

            {hasChanges && (
              <div className="mt-2 flex flex-col items-center gap-2">
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
                  disabled={disabled || !isValid || !canSave || pending}
                  onClick={confirm}>
                  {pending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-3 h-3" />} Save
                </Button>
              </div>
            )}
          </div>
        </aside>

        {/* Main */}
        <main className="flex-1 flex flex-col gap-4">
          {sections.map((sectionKey) => {
            const section = schema[sectionKey];

            return (
              <div key={sectionKey} className="flex flex-col gap-5 border-b last:border-b-0 pb-10 last:pb-4">
                {section.title && <div className="text-md font-bold uppercase">{section.title}</div>}

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
                          'relative border rounded-sm p-6 scroll-mt-22 xl:scroll-mt-20 shadow-xs',
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
                        className="relative border rounded-md p-6 scroll-mt-22 xl:scroll-mt-20">
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
                      className={`relative rounded-sm border border-border/70 bg-background p-6 shadow-xs ${group.direction === 'horizontal' ? 'flex-row' : 'flex-col'} scroll-mt-22 xl:scroll-mt-20`}>
                      <div className="flex flex-col gap-4 w-full">
                        {(group.title || group.description || group.requiredLicense) && (
                          <div className="flex items-start justify-between gap-3 border-b border-dashed border-border/70 pb-4">
                            <div className="flex min-w-0 flex-col gap-1.5">
                              {group.title && (
                                <h3 className="text-sm font-semibold tracking-tight text-foreground/90">
                                  {group.title}
                                </h3>
                              )}
                              {typeof group.description === 'string' ? (
                                <p className="max-w-full text-sm leading-6 text-muted-foreground">{group.description}</p>
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
                                  `relative pb-6 last:pb-0 ${group.direction === 'horizontal' ? 'flex-1' : 'block border-b last:border-b-0'}`,
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
      <div className="xl:hidden sticky bottom-0 bg-background border-t pt-3 pb-3 mt-2">
        <div className="flex justify-end gap-2">
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
          <Button size="sm" onClick={confirm} disabled={disabled || !isValid || !canSave || pending}>
            {pending ? <Loader2 className="w-4 h-4 animate-spin mr-1" /> : <Save className="w-3 h-3 mr-1" />} Save
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
