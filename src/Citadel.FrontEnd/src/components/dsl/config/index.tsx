import { cn } from '@/lib/utils';
import { AlertTriangle, History, Settings } from 'lucide-react';
import { Fragment, ReactNode, SetStateAction } from 'react';
import { Button } from '../../ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select';
import { Input } from '@/components/ui/input';

const keys = <T extends Record<string, unknown>>(obj: T) => Object.keys(obj) as Array<keyof T>;

export const ConfigLayout = <T,>({
  original,
  update,
  children,
  disabled,
  onConfirm,
  onReset,
  selector,
  titleOther,
}: {
  original: T;
  update: Partial<T>;
  children: ReactNode;
  disabled: boolean;
  onConfirm: () => void;
  onReset: () => void;
  selector?: ReactNode;
  titleOther?: ReactNode;
}) => {
  const titleProps = titleOther ? { titleOther } : { title: 'Config', icon: <Settings className="w-4 h-4" /> };
  const changesMade = Object.keys(update).length ? true : false;
  return (
    <Section
      {...titleProps}
      actions={
        <div className="flex gap-2">
          {changesMade && (
            <div className="text-muted-foreground flex items-center gap-2">
              <AlertTriangle className="w-4 h-4" /> Unsaved changes
              <AlertTriangle className="w-4 h-4" />
            </div>
          )}
          {selector}
          {changesMade && (
            <>
              <Button
                variant="outline"
                onClick={onReset}
                disabled={disabled || !changesMade}
                className="flex items-center gap-2">
                <History className="w-4 h-4" />
                Reset
              </Button>
            </>
          )}
        </div>
      }>
      {children}
    </Section>
  );
};

export type PrimitiveConfigArgs = {
  placeholder?: string;
  label?: string;
  boldLabel?: boolean;
  description?: ReactNode;
};

export type ConfigComponent<T> = {
  label: string;
  boldLabel?: boolean; // defaults to true
  icon?: ReactNode;
  actions?: ReactNode;
  labelExtra?: ReactNode;
  description?: ReactNode;
  hidden?: boolean;
  labelHidden?: boolean;
  contentHidden?: boolean;
  components: {
    [K in keyof Partial<T>]:
      | boolean
      | PrimitiveConfigArgs
      | ((value: T[K], set: (value: Partial<T>) => void) => ReactNode);
  };
};

export const Config = <T,>({
  original,
  update,
  disabled,
  disableSidebar,
  set,
  onSave,
  components,
  selector,
  titleOther,
}: {
  original: T;
  update: Partial<T>;
  disabled: boolean;
  disableSidebar?: boolean;
  set: React.Dispatch<SetStateAction<Partial<T>>>;
  onSave: () => Promise<void>;
  selector?: ReactNode;
  titleOther?: ReactNode;
  components: Record<
    string, // sidebar key
    ConfigComponent<T>[] | false | undefined
  >;
}) => {
  const sections = keys(components).filter((section) => !!components[section]);
  const changesMade = Object.keys(update).length ? true : false;
  const onConfirm = async () => {
    await onSave();
    set({});
  };
  const onReset = () => set({});
  return (
    <ConfigLayout
      original={original}
      titleOther={titleOther}
      update={update}
      disabled={disabled}
      onConfirm={onConfirm}
      onReset={onReset}
      selector={selector}>
      <div className="flex gap-6">
        {!disableSidebar && (
          <div className="hidden xl:block relative pr-6 border-r">
            <div className="sticky top-24 hidden xl:flex flex-col gap-8 w-[140px] h-fit pb-24">
              {sections.map((section) => (
                <div key={section}>
                  {section && <p className="text-muted-foreground uppercase text-right mb-2">{section}</p>}
                  <div className="flex flex-col gap-2">
                    {components[section] &&
                      components[section]
                        .filter((item) => !item.hidden)
                        .map((item) => (
                          <a href={'#' + section + item.label} key={section + item.label}>
                            <Button variant="secondary" className="justify-end w-full" size="sm">
                              {item.label}
                            </Button>
                          </a>
                        ))}
                  </div>
                </div>
              ))}
              {changesMade && (
                <div className="flex flex-col gap-2">
                  <Button
                    variant="outline"
                    onClick={onReset}
                    disabled={disabled || !changesMade}
                    className="flex items-center gap-2">
                    <History className="w-4 h-4" />
                    Reset
                  </Button>
                </div>
              )}
            </div>
          </div>
        )}
        <div className="w-full flex flex-col gap-12">
          {sections.map(
            (section) =>
              components[section] && (
                <div key={section} className="relative pb-12 border-b last:pb-0 last:border-b-0 ">
                  <div className="xl:hidden sticky top-16 h-16 flex items-center justify-between bg-background z-10">
                    {section && <p className="uppercase text-2xl">{section}</p>}
                    <Select onValueChange={(value) => (window.location.hash = value)}>
                      <SelectTrigger className="w-32 capitalize xl:hidden">
                        <SelectValue placeholder="Go To" />
                      </SelectTrigger>
                      <SelectContent className="w-32">
                        {components[section]
                          .filter((item) => !item.hidden)
                          .map(({ label }) => (
                            <SelectItem key={section + label} value={section + label} className="capitalize">
                              {label}
                            </SelectItem>
                          ))}
                      </SelectContent>
                    </Select>
                  </div>
                  {section && <p className="hidden xl:block bg-background text-2xl uppercase mb-6 h-fit">{section}</p>}
                  <div className="flex flex-col gap-6 w-full">
                    {components[section].map(
                      ({
                        label,
                        boldLabel = true,
                        labelHidden,
                        icon,
                        labelExtra,
                        actions,
                        description,
                        hidden,
                        contentHidden,
                        components,
                      }) => (
                        <div
                          key={section + label}
                          id={section + label}
                          className={cn(
                            'p-6 border rounded-md flex flex-col gap-6 scroll-mt-40 xl:scroll-mt-24',
                            hidden && 'hidden',
                          )}>
                          {!labelHidden && (
                            <div className="flex justify-between">
                              <div>
                                <div className="flex items-center gap-4">
                                  {icon}
                                  <div className={cn('text-lg', boldLabel && 'font-bold')}>{label}</div>
                                  {labelExtra}
                                </div>
                                {description && <div className="text-sm text-muted-foreground">{description}</div>}
                              </div>
                              {actions}
                            </div>
                          )}
                          {!contentHidden && (
                            <div className="flex flex-col gap-4">
                              {keys(components).map((key) => {
                                const renderer = components[key];
                                if (typeof renderer === 'function') {
                                  const value = (update[key] ?? original[key]) as T[keyof T];
                                  return (
                                    <Fragment key={key as string}>
                                      {renderer(value, (newValue) => set((prev) => ({ ...prev, ...newValue })))}
                                    </Fragment>
                                  );
                                }
                                return null;
                              })}
                            </div>
                          )}
                        </div>
                      ),
                    )}
                  </div>
                </div>
              ),
          )}
          {changesMade && (
            <div className="flex gap-2 justify-end">
              <div className="text-muted-foreground flex items-center gap-2">
                <AlertTriangle className="w-4 h-4" /> Unsaved changes
                <AlertTriangle className="w-4 h-4" />
              </div>
              <Button variant="outline" onClick={onReset} disabled={disabled} className="flex items-center gap-2">
                <History className="w-4 h-4" />
                Reset
              </Button>
            </div>
          )}
        </div>
      </div>
    </ConfigLayout>
  );
};

interface SectionProps {
  title?: ReactNode;
  icon?: ReactNode;
  titleRight?: ReactNode;
  titleOther?: ReactNode;
  children?: ReactNode;
  actions?: ReactNode;
  // otherwise items-start
  itemsCenterTitleRow?: boolean;
  className?: string;
}

export const Section = ({
  title,
  icon,
  titleRight,
  titleOther,
  actions,
  children,
  itemsCenterTitleRow,
  className,
}: SectionProps) => (
  <div className={cn('flex flex-col gap-4', className)}>
    {(title || icon || titleRight || titleOther || actions) && (
      <div className={cn('flex flex-wrap gap-2 justify-between', itemsCenterTitleRow ? 'items-center' : 'items-start')}>
        {title || icon ? (
          <div className="px-2 flex items-center gap-2 text-muted-foreground">
            {icon}
            {title && <h2 className="text-xl">{title}</h2>}
            {titleRight}
          </div>
        ) : (
          titleOther
        )}
        {actions}
      </div>
    )}
    {children}
  </div>
);

export const ConfigInput = ({
  label,
  boldLabel,
  value,
  description,
  disabled,
  placeholder,
  onChange,
  onBlur,
  className,
  inputLeft,
  inputRight,
}: {
  label: string;
  boldLabel?: boolean;
  value: string | number | undefined;
  description?: ReactNode;
  disabled?: boolean;
  placeholder?: string;
  onChange?: (value: string) => void;
  onBlur?: (value: string) => void;
  className?: string;
  inputLeft?: ReactNode;
  inputRight?: ReactNode;
}) => (
  <ConfigItem label={label} boldLabel={boldLabel} description={description}>
    {inputLeft || inputRight ? (
      <div className="flex gap-2 items-center">
        {inputLeft}
        <Input
          className={cn('max-w-[75%] lg:max-w-[400px]', className)}
          type={typeof value === 'number' ? 'number' : undefined}
          value={value}
          onChange={(e) => onChange && onChange(e.target.value)}
          onBlur={(e) => onBlur && onBlur(e.target.value)}
          placeholder={placeholder}
          disabled={disabled}
        />
        {inputRight}
      </div>
    ) : (
      <Input
        className={cn('max-w-[75%] lg:max-w-[400px]', className)}
        type={typeof value === 'number' ? 'number' : undefined}
        value={value}
        onChange={(e) => onChange && onChange(e.target.value)}
        onBlur={(e) => onBlur && onBlur(e.target.value)}
        placeholder={placeholder}
        disabled={disabled}
      />
    )}
  </ConfigItem>
);

export const ConfigItem = ({
  label,
  boldLabel,
  description,
  children,
  className,
}: {
  label?: ReactNode;
  boldLabel?: boolean;
  description?: ReactNode;
  children: ReactNode;
  className?: string;
}) => (
  <div className={cn('pb-6 border-b flex flex-col gap-4 first:pt-0 last:border-b-0 last:pb-0', className)}>
    {(label || description) && (
      <div>
        {label && typeof label === 'string' && (
          <div className={cn('capitalize', boldLabel && 'font-bold')}>{label.split('_').join(' ')}</div>
        )}
        {label && typeof label !== 'string' && label}
        {description && <div className="text-sm text-muted-foreground">{description}</div>}
      </div>
    )}
    {children}
  </div>
);
