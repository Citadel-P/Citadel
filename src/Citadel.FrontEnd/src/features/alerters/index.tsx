import { Megaphone, Plus, Rss, LoaderCircle, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { AlertRulesTable } from './table';
import { useMutate, useRead } from '@/lib/hooks';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { AlertDestination, AlertChannelInput, AlertChannelView, AlertRuleView } from '@/api/generated/api.types';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Separator } from '@radix-ui/react-dropdown-menu';
import { AlertRuleDropdownActions, AlertRuleGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';
import { Label } from '@/components/ui/label';
import { toast } from 'sonner';
import { FieldInput, FieldSwitch, ItemSelector } from '@/components/custom/form-builder';

const EMPTY_CHANNEL: AlertChannelInput = {
  name: '',
  alertDestination: AlertDestination.Slack,
  url: '',
  isActive: true,
};

const urlFormatHelper: Record<string, string> = {
  Generic: 'generic://example.com?template=json',
  Bark: 'bark://devicekey@host',
  Discord: 'discord://token@id',
  Gotify: 'gotify://gotify-host/token',
  GoogleChat: 'googlechat://chat.googleapis.com/v1/spaces/FOO/messages?key=bar&token=baz',
  IFTTT: 'ifttt://key/?events=event1[,event2,...]&value1=value1&value2=value2&value3=value3',
  Join: 'join://shoutrrr:api-key@join/?devices=device1[,device2,...][&icon=icon]',
  Lark: 'lark://host/token?secret=secret',
  Mattermost: 'mattermost://[username@]mattermost-host/token[/channel]',
  Matrix: 'matrix://username:password@host:port/[?rooms=!roomID1[,roomAlias2]]',
  Ntfy: 'ntfy://:accesstoken@host/topic',
  OpsGenie: 'opsgenie://host/token?responders=responder1[,responder2]_',
  Pushbullet: 'pushbullet://api-token[/device/#channel/email]',
  Pushover: 'pushover://shoutrrr:apiToken@userKey/?devices=device1[,device2,...]',
  Rocketchat: 'rocketchat://[username@]rocketchat-host/token[/channel|@recipient]',
  Signal: 'signal://[user[:password]@]host[:port]/source_phone/recipient1[,recipient2,...]',
  Slack: 'slack://[botname@]token-a/token-b/token-c',
  Teams: 'teams://group@tenant/altId/groupOwner/extraId?host=organization.webhook.office.com',
  Telegram: 'telegram://token@telegram?chats=@channel-1[,chat-id-1,...]',
  WeCom: 'wecom://key',
  ZulipChat: 'zulip://bot-mail:bot-key@zulip-domain/?stream=name-or-id&topic=name',
};

const getInitialInput = (c?: AlertChannelView): AlertChannelInput =>
  c
    ? {
        name: c.name,
        alertDestination: c.alertDestination,
        url: c.url,
        isActive: c.isActive,
      }
    : { ...EMPTY_CHANNEL };

const getDestinationColor = (dest: AlertDestination) => {
  switch (dest) {
    case AlertDestination.Slack:
      return 'bg-[#4A154B] border-[#4A154B]/10';
    case AlertDestination.Discord:
      return 'bg-[#5865F2] border-[#5865F2]/10';
    case AlertDestination.Generic:
      return 'bg-blue-600 border-blue-600/10';
    default:
      return 'bg-zinc-900 border-zinc-900/10';
  }
};

function useAlertChannels() {
  const { data, refetch } = useRead('listAlertChannels');
  const channels = data?.data?.channels ?? [];

  const { mutateAsync: verify, isPending: verifying } = useMutate('verifyAlertChannel');
  const { mutateAsync: create, isPending: creating } = useMutate('createAlertChannel');
  const { mutateAsync: update, isPending: updating } = useMutate('updateAlertChannel');
  const { mutateAsync: deleteChannels, isPending: deleting } = useMutate('deleteAlertChannels');

  const save = async (editing: AlertChannelView | null, input: AlertChannelInput) => {
    if (!input.name) throw new Error('Channel name is required.');
    if (!input.url) throw new Error('Channel URL is required.');

    if (editing) {
      await update({ id: editing.id, data: input });
    } else {
      await create({ data: input });
    }

    await refetch();
  };

  const removeChannel = async (id: string) => {
    await deleteChannels({ ids: [id] } as any);
    await refetch();
  };

  const verifyChannel = async (dest: AlertDestination, url: string) => {
    if (!url) {
      toast.error('Channel URL is required to verify.');
      throw new Error('Channel URL is required to verify.');
    }
    await verify({ data: { alertDestination: dest, url } });
  };

  return {
    channels,
    save,
    verifyChannel,
    removeChannel,
    verifying,
    saving: creating || updating,
    deleting,
  };
}

function ChannelCard({
  channel,
  onEdit,
  onDelete,
}: {
  channel: AlertChannelView;
  onEdit: () => void;
  onDelete: (id: string) => Promise<void>;
}) {
  const color = getDestinationColor(channel.alertDestination);
  const letter = channel.alertDestination.charAt(0).toUpperCase();

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onEdit}
      onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && onEdit()}
      className="group relative bg-background rounded-xl border border-muted p-4 hover:border-zinc-300 hover:shadow-md transition-all cursor-pointer flex flex-col justify-between h-[140px]">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              'h-10 w-10 rounded-lg flex items-center justify-center border shadow-sm text-white font-bold',
              color,
            )}>
            {letter}
          </div>
          <div>
            <h3 className="font-semibold text-sm">{channel.name}</h3>
            <p className="text-xs text-muted-foreground truncate">{channel.alertDestination}</p>
          </div>
        </div>
      </div>

      <div className="flex items-center justify-between mt-4 pt-4 border-t border-muted">
        <div className="flex items-center gap-1.5">
          <div
            className={cn(
              'h-2 w-2 rounded-full',
              channel.isActive ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]' : 'bg-zinc-300',
            )}
          />
          <span className="text-xs font-medium text-zinc-600">{channel.isActive ? 'Active' : 'Inactive'}</span>
        </div>
        <div onClick={(e) => e.stopPropagation()} onKeyDown={(e) => e.stopPropagation()} role="presentation">
          <ActionWithDialog
            name={channel.name}
            title="Delete"
            icon={<Trash2 className="h-3.5 w-3.5" />}
            variant="outline"
            onClick={() => onDelete(channel.id)}
          />
        </div>
      </div>
    </div>
  );
}

function AlertNotificationChannels() {
  const { channels, save, verifyChannel, saving, verifying, removeChannel } = useAlertChannels();

  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<AlertChannelView | null>(null);
  const [input, setInput] = useState<AlertChannelInput>(EMPTY_CHANNEL);

  const openAdd = () => {
    setEditing(null);
    setInput(getInitialInput());
    setOpen(true);
  };

  const openEdit = (c: AlertChannelView) => {
    setEditing(c);
    setInput(getInitialInput(c));
    setOpen(true);
  };

  const handleSave = async () => {
    try {
      await save(editing, input);
      toast.success(`Channel ${editing ? 'updated' : 'created'} successfully!`);
      setOpen(false);
    } catch (e: any) {
      toast.error(e.message ?? 'Operation failed.');
    }
  };

  const handleVerify = async () => {
    await verifyChannel(input.alertDestination, input.url);
    toast.success('Channel verified successfully!');
  };

  const handleDelete = async (id: string) => {
    try {
      await removeChannel(id);
      toast.success('Channel deleted successfully!');
    } catch (e: any) {
      toast.error(e.message ?? 'Failed to delete channel.');
      throw e;
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex justify-between">
        <div className="flex items-center gap-3">
          <div className="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Rss className="h-4 w-4" />
          </div>
          <div>
            <div className="font-bold">Notification Channels</div>
            <p className="text-xs text-muted-foreground">Destinations for your alert notifications.</p>
          </div>
        </div>
        <Button variant="outline" onClick={openAdd}>
          <Plus className="h-3 w-3" /> Add Channel
        </Button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4">
        {channels.map((c) => (
          <ChannelCard key={c.id} channel={c} onEdit={() => openEdit(c)} onDelete={handleDelete} />
        ))}

        <button
          onClick={openAdd}
          className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed p-4 text-zinc-400 hover:text-zinc-600 h-[140px]">
          <Plus className="h-5 w-5" />
          <span className="text-sm font-medium">Connect New Channel</span>
        </button>
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-[550px]" onInteractOutside={(e) => e.preventDefault()}>
          <DialogHeader>
            <DialogTitle>{editing ? 'Edit' : 'Add'} Notification Channel</DialogTitle>
            <DialogDescription>
              {editing ? 'Update your notification channel.' : 'Connect a new service to receive alerts.'}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div>
              <Label>Name</Label>
              <FieldInput
                className="max-w-full"
                placeholder="e.g. Engineering Team Channel"
                value={input.name}
                onChange={(v) => setInput({ ...input, name: v })}
              />
            </div>
            <div>
              <Label>Type</Label>
              <ItemSelector
                className="max-w-full"
                collection={AlertDestination as any}
                value={input.alertDestination}
                onChange={(v) => setInput({ ...input, alertDestination: v })}
              />
            </div>
            <div>
              <Label htmlFor="channelURL">Channel URL</Label>

              <Label>Channel URL</Label>
              <FieldInput
                id="channelURL"
                className="max-w-full"
                value={input.url}
                placeholder={`${urlFormatHelper[input.alertDestination]?.split('://')[0] ?? 'https'}://...`}
                onChange={(v) => setInput({ ...input, url: v })}
              />
              <div className="text-xs text-muted-foreground mt-1">
                Format:{' '}
                <span className="font-mono bg-muted p-0.5 rounded">
                  {urlFormatHelper[input.alertDestination] ?? 'url'}
                </span>
              </div>
            </div>

            <div className="flex flex-row items-center justify-between rounded-lg border p-3 shadow-xs border-border">
              <div className="space-y-0.5">
                <Label htmlFor="is-active">Is Active</Label>
                <div className="text-xs text-muted-foreground">
                  Determine if alerts should be sent to this channel immediately.
                </div>
              </div>
              <div>
                <FieldSwitch
                  id="is-active"
                  checked={input.isActive}
                  onChange={(v) => setInput({ ...input, isActive: v })}
                />
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)} disabled={saving || verifying}>
              Cancel
            </Button>
            <Button variant="outline" onClick={handleVerify} disabled={verifying || saving}>
              Verify {verifying && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
            </Button>
            <Button onClick={handleSave} disabled={saving || verifying}>
              Save {saving && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export const AlertRuleComponents: RequiredComponents = {
  Icon: <Megaphone className="h-4 w-4" />,
  Content: ({ items, actions, isLoading }) => (
    <div className="flex flex-col gap-6">
      <AlertRulesTable items={items} actions={actions} isLoading={isLoading} />
      <Separator className="border-b-1 border-dashed" />
      <AlertNotificationChannels />
    </div>
  ),
  DropdownActions: AlertRuleDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="AlertRule" items={items} actions={Object.values(AlertRuleGroupActions)} />
  ),
  header: {
    title: 'Alert Rules',
    subtitle: 'Manage conditions and thresholds.',
    showSearch: true,
    showAdd: true,
    addButtonTitle: 'Add Rule',
  },
  useData(): ResourceDataHookResult<AlertRuleView> {
    const { data, isLoading } = useRead('listAlertRules');
    return { items: data?.data.alertRules ?? [], isLoading };
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
