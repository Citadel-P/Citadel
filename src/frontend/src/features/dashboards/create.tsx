import { useState } from 'react';
import { useNavigate } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { type DashboardConfiguration, DashboardVisibility, LicenseCapability } from '@/api/generated/api.types';
import { MarkdownDescriptionEditor } from '@/components/custom/markdown-description';
import { FieldInput } from '@/components/custom/form-builder';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { useMutate, useRead } from '@/lib/hooks';
import { requestErrorMessage } from '@/lib/request-error';
import { randomUuid } from '@/lib/utils';
import { dashboardTemplates, fromTemplate, type DashboardTemplate } from './templates';
import { ScopeField } from './display-controls';
import { MetricSourcePicker } from './widget-settings';
import { widgetError } from './catalogue';
import { audiences } from './visibility';

export function CreateDashboardDialog({
  close,
  initialTemplate = 'Blank',
  copy,
}: {
  close: () => void;
  initialTemplate?: DashboardTemplate;
  copy?: { name: string; configuration: DashboardConfiguration };
}) {
  const navigate = useNavigate();
  const client = useQueryClient();
  const [id] = useState(randomUuid);
  const [name, setName] = useState(copy ? ('Copy of ' + copy.name).slice(0, 128) : '');
  const [description, setDescription] = useState(copy?.configuration.description ?? '');
  const [template, setTemplate] = useState<DashboardTemplate>(initialTemplate);
  // Keep widget IDs stable if a committed create loses its response and the user retries.
  const [configuration, setConfiguration] = useState(() =>
    copy
      ? {
          ...structuredClone(copy.configuration),
          widgets: copy.configuration.widgets.map((w) => ({ ...structuredClone(w), id: randomUuid() })),
        }
      : fromTemplate(initialTemplate),
  );
  const [audience, setAudience] = useState(DashboardVisibility.Private);
  const profile = useRead('getCurrentProfile');
  const admin = !profile.isError && profile.data?.data.authorization.isAdministrator;
  const { hasCapability } = useLicenseEntitlements();
  const canRestrict = hasCapability(LicenseCapability.CustomAccessControl);
  const permitted =
    audience === DashboardVisibility.Private ||
    (admin && (audience === DashboardVisibility.InstanceShared || canRestrict));
  const create = useMutate('createDashboard', {
    onSuccess: ({ data }) => {
      void client.invalidateQueries({ queryKey: ['listDashboards'], refetchType: 'none' });
      close();
      navigate('/dashboards/' + data.summary.id);
    },
  });
  const trimmed = name.trim();
  const nameLength = [...trimmed].length;
  const metric = configuration.widgets.find((w) => w.settings.kind === 'Metrics')?.settings;
  const content = copy
    ? configuration
    : {
        ...configuration,
        widgets: configuration.widgets.filter(
          (w) =>
            (w.settings.kind !== 'Metrics' || !!w.settings.source.id) &&
            (admin || (w.settings.kind !== 'UpcomingSchedules' && w.settings.kind !== 'Updates')),
        ),
      };
  const canSubmit =
    !content.widgets.some(widgetError) && permitted && nameLength > 0 && nameLength <= 128 && !create.isPending;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !create.isPending) close();
      }}>
      <DialogContent showCloseButton={!create.isPending}>
        <DialogHeader>
          <DialogTitle>Create dashboard</DialogTitle>
          <DialogDescription>Choose a name and starting layout.</DialogDescription>
        </DialogHeader>
        <form
          className="grid min-w-0 gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (canSubmit)
              create.mutate({
                data: {
                  id,
                  name: trimmed,
                  visibility: audience,
                  configuration: { ...content, description: description.trim() || undefined },
                },
              });
          }}>
          <div className="grid min-w-0 gap-2">
            <Label htmlFor="dashboard-name">Name</Label>
            <FieldInput
              className="w-full max-w-none"
              id="dashboard-name"
              value={name}
              onChange={setName}
              disabled={create.isPending}
            />
            {nameLength > 128 && <p role="alert">Use up to 128 characters.</p>}
          </div>
          <div className="grid min-w-0 gap-2">
            <Label htmlFor="dashboard-description">Description</Label>
            <MarkdownDescriptionEditor
              id="dashboard-description"
              value={description}
              maxLength={4096}
              disabled={create.isPending}
              onChange={(event) => setDescription(event.target.value)}
            />
          </div>
          {!copy && (
            <div className="grid min-w-0 gap-2">
              <Label htmlFor="dashboard-template">Template</Label>
              <Select
                value={template}
                disabled={create.isPending}
                onValueChange={(value: DashboardTemplate) => {
                  setTemplate(value);
                  setConfiguration(fromTemplate(value));
                }}>
                <SelectTrigger className="w-full" id="dashboard-template">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {dashboardTemplates.map((item) => (
                    <SelectItem key={item} value={item}>
                      {item}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
          {!copy && template === 'Application' && (
            <div className="grid gap-2 [&_button[aria-haspopup=dialog]]:w-full">
              <Label>Match all tags</Label>
              <ScopeField value={configuration.tags} change={(tags) => setConfiguration({ ...configuration, tags })} />
            </div>
          )}
          {!copy && metric?.kind === 'Metrics' && (
            <MetricSourcePicker
              source={metric.source}
              change={(source) =>
                setConfiguration({
                  ...configuration,
                  widgets: configuration.widgets.map((w) =>
                    w.settings.kind === 'Metrics' ? { ...w, settings: { ...w.settings, source } } : w,
                  ),
                })
              }
            />
          )}
          {admin && (
            <div className="grid min-w-0 gap-2">
              <Label htmlFor="dashboard-audience">Who can read</Label>
              <Select
                value={audience}
                onValueChange={(value: DashboardVisibility) => setAudience(value)}
                disabled={create.isPending}>
                <SelectTrigger className="w-full" id="dashboard-audience">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={DashboardVisibility.Private}>Private</SelectItem>
                  <SelectItem value={DashboardVisibility.InstanceShared}>{audiences.InstanceShared}</SelectItem>
                  {canRestrict && <SelectItem value={DashboardVisibility.Restricted}>Restricted</SelectItem>}
                </SelectContent>
              </Select>
            </div>
          )}
          {audience === DashboardVisibility.Private && (
            <p className="text-sm text-muted-foreground">
              Only you can open this dashboard. Personal dashboards always stay private.
            </p>
          )}
          {create.isError && (
            <p role="alert" className="text-sm text-destructive">
              {requestErrorMessage(create.error)}
            </p>
          )}
          <DialogFooter>
            <Button type="button" variant="outline" disabled={create.isPending} onClick={close}>
              Cancel
            </Button>
            <Button type="submit" disabled={!canSubmit}>
              Create
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
