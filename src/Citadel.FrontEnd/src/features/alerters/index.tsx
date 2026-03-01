import { Megaphone, MoreHorizontal, Plus, Rss } from 'lucide-react';
import { useState } from 'react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { AlertRulesTable } from './table';
import { useRead } from '@/lib/hooks';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { AlertChannelView } from '@/api/generated/api.types';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Separator } from '@radix-ui/react-dropdown-menu';
import { AlertRuleDropdownActions, AlertRuleGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';

export const AlerterComponents: RequiredComponents = {
  Icon: <Megaphone className="h-4 w-4" />,
  Content: ({ items, actions, isLoading }) => {
    return (
      <div className="flex flex-col gap-6">
        <AlertRulesTable pagedResult={items as any} actions={actions} isLoading={isLoading} displayPagging={true} />
        <Separator className="border-b-1 border-dashed " />
        <AlertNotificationChannels />
      </div>
    );
  },
  DropdownActions: AlertRuleDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Alerter" items={items} actions={Object.values(AlertRuleGroupActions)} />;
  },
  header: {
    title: 'Alert Rules',
    subtitle: 'Manage conditions and thresholds.',
    showSearch: false,
    showAdd: true,
    addButtonTitle: 'Add Rule',
  },
  useData: function (): ResourceDataHookResult<any> {
    const { data, isLoading } = useRead('listAlertRules');
    return { items: data?.data.pagedResult, isLoading };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (v) =>
        v.name?.toLowerCase().includes(s) ||
        v.id?.toLowerCase().includes(s) ||
        v.id?.substring(0, 12).toLowerCase().includes(s),
    );
  },
};

function AlertNotificationChannels() {
  const { data, isLoading } = useRead('listAlertChannels');
  const [channels, setChannels] = useState<AlertChannelView[]>(data?.data?.channels ?? []);
  const [isChannelDialogOpen, setIsChannelDialogOpen] = useState(false);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col sm:flex-row gap-2 sm:justify-between">
        <div className="flex items-center gap-3">
          <div className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Rss className="h-4 w-4" />
            <span className="sr-only">Notification Channels</span>
          </div>
          <div className="flex flex-col">
            <div className="text-md font-bold text-foreground">Notification Channels</div>
            <p className="text-xs text-muted-foreground">Destinations for your alert notifications.</p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button
            type="button"
            variant={'outline'}
            onClick={() => setIsChannelDialogOpen(true)}
            className="inline-flex items-center rounded-sm text-sm px-2.5 py-2.5">
            <Plus className="h-3 w-3" /> Add Channel
          </Button>
        </div>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4">
        {channels.map((channel) => (
          <div
            key={channel.id}
            className="group relative bg-white rounded-xl border border-zinc-200 p-4 hover:border-zinc-300 hover:shadow-md transition-all cursor-pointer flex flex-col justify-between h-[140px]">
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <div
                  className={cn(
                    'h-10 w-10 rounded-lg flex items-center justify-center border shadow-sm',
                    channel.alertDestination === 'Slack'
                      ? 'bg-[#4A154B] border-[#4A154B]/10 text-white'
                      : channel.alertDestination === 'Discord'
                        ? 'bg-[#5865F2] border-[#5865F2]/10 text-white'
                        : channel.alertDestination === 'Generic'
                          ? 'bg-blue-600 border-blue-600/10 text-white'
                          : 'bg-zinc-900 border-zinc-900/10 text-white',
                  )}></div>
                <div>
                  <h3 className="font-semibold text-sm text-zinc-900">{channel.alertDestination}</h3>
                  <p className="text-xs text-zinc-500 truncate max-w-[120px]">{channel.url}</p>
                </div>
              </div>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 -mr-2 -mt-2 opacity-0 group-hover:opacity-100 transition-opacity text-zinc-400 hover:text-zinc-900">
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </div>

            <div className="flex items-center justify-between mt-4 pt-4 border-t border-zinc-50">
              <div className="flex items-center gap-1.5">
                <div
                  className={cn(
                    'h-2 w-2 rounded-full',
                    channel.isActive ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]' : 'bg-zinc-300',
                  )}
                />
                <span className="text-xs font-medium text-zinc-600">{channel.isActive ? 'Active' : 'Inactive'}</span>
              </div>
              <span className="text-[10px] text-zinc-400 font-medium">ID: {channel.id.substring(0, 4)}</span>
            </div>
          </div>
        ))}

        {/* Add New Placeholder Card */}
        <button
          onClick={() => setIsChannelDialogOpen(true)}
          className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-zinc-300 p-4 hover:bg-zinc-50 hover:border-zinc-400 transition-all text-zinc-400 hover:text-zinc-600 h-[140px] group">
          <div className="h-10 w-10 rounded-full bg-zinc-100 group-hover:bg-zinc-200 flex items-center justify-center transition-colors">
            <Plus className="h-5 w-5" />
          </div>
          <span className="text-sm font-medium">Connect New Channel</span>
        </button>
      </div>
      <Dialog open={isChannelDialogOpen} onOpenChange={setIsChannelDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Notification Channel</DialogTitle>
            <DialogDescription>Connect a new service to receive alerts.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <span>Channel Name</span>
              <Input placeholder="e.g. Engineering Team Slack" />
            </div>
            <div className="space-y-2">
              <span>Type</span>
              <Select defaultValue="Slack">
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Slack">Slack</SelectItem>
                  <SelectItem value="Discord">Discord</SelectItem>
                  <SelectItem value="Webhook">Webhook</SelectItem>
                  <SelectItem value="Email">Email</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setIsChannelDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => {
                setIsChannelDialogOpen(false);
              }}>
              Add Channel
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
