import LogoIcon from '@/assets/logo.svg';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { Copy, Download, LoaderCircle } from 'lucide-react';
import QRCode from 'qrcode';
import { FormEvent, ReactNode, useEffect, useMemo, useState } from 'react';

export function MfaAuthShell({
  title,
  description,
  children,
  panelClassName,
}: {
  title: string;
  description?: string;
  children: ReactNode;
  panelClassName?: string;
}) {
  return (
    <div className="min-h-screen w-full overflow-auto bg-card">
      <div className="mx-auto flex min-h-screen flex-col items-center justify-center px-6 py-8">
        <div className="mb-6 flex items-start">
          <span className="mr-2 h-9 w-9 rounded bg-primary p-2 text-background">
            <LogoIcon />
          </span>
          <span className="text-2xl font-semibold">Citadel</span>
        </div>

        <div className={cn('w-full rounded-lg bg-background shadow-sm sm:max-w-md', panelClassName)}>
          <div className="space-y-5 p-6 sm:p-8">
            <div className="space-y-2">
              <h1 className="text-xl font-bold leading-tight md:text-2xl">{title}</h1>
              {description && <p className="text-sm leading-6 text-muted-foreground">{description}</p>}
            </div>
            {children}
          </div>
        </div>
      </div>
    </div>
  );
}

export function TotpCodeForm({
  label = 'Authenticator code',
  pending,
  error,
  submitLabel,
  onSubmit,
  footer,
}: {
  label?: string;
  pending: boolean;
  error?: string;
  submitLabel: string;
  onSubmit: (code: string) => void | Promise<void>;
  footer?: ReactNode;
}) {
  const [code, setCode] = useState('');
  const canSubmit = /^\d{6}$/.test(code);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (!canSubmit) return;
    await onSubmit(code);
  };

  return (
    <form className="space-y-4" onSubmit={handleSubmit}>
      {error && <AlertMessage type="warning">{error}</AlertMessage>}
      <div className="space-y-2">
        <label className="text-sm font-medium leading-none" htmlFor="mfa-code">
          {label}
        </label>
        <Input
          id="mfa-code"
          inputMode="numeric"
          autoComplete="one-time-code"
          maxLength={6}
          value={code}
          onChange={(event) => setCode(event.target.value.replace(/\D/g, '').slice(0, 6))}
          placeholder="123456"
          disabled={pending}
        />
      </div>
      <Button type="submit" className="w-full" disabled={!canSubmit || pending}>
        {submitLabel}
        {pending && <LoaderCircle className="ml-1 size-4 animate-spin" />}
      </Button>
      {footer}
    </form>
  );
}

export function TotpSetupPanel({ secret, otpAuthUri }: { secret: string; otpAuthUri: string }) {
  const [qrDataUrl, setQrDataUrl] = useState<string>();

  useEffect(() => {
    let cancelled = false;
    QRCode.toDataURL(otpAuthUri, { margin: 1, width: 192 })
      .then((value) => {
        if (!cancelled) setQrDataUrl(value);
      })
      .catch(() => setQrDataUrl(undefined));

    return () => {
      cancelled = true;
    };
  }, [otpAuthUri]);

  return (
    <div className="space-y-4">
      <div className="flex justify-center rounded-sm border bg-white p-4">
        {qrDataUrl ? (
          <img src={qrDataUrl} alt="Authenticator QR code" className="h-48 w-48" />
        ) : (
          <div className="h-48 w-48" />
        )}
      </div>
      <div className="space-y-2">
        <div className="text-sm font-medium leading-none">Manual secret</div>
        <div className="break-all rounded-sm border bg-muted/30 px-3 py-2 font-mono text-sm">{secret}</div>
      </div>
    </div>
  );
}

export function RecoveryCodesPanel({
  codes,
  actionLabel,
  onContinue,
}: {
  codes: ReadonlyArray<string>;
  actionLabel: string;
  onContinue: () => void;
}) {
  const [confirmed, setConfirmed] = useState(false);
  const text = useMemo(() => codes.join('\n'), [codes]);

  const copyCodes = async () => {
    await navigator.clipboard.writeText(text);
  };

  const downloadCodes = () => {
    const blob = new Blob([text], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = 'citadel-recovery-codes.txt';
    anchor.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-2 rounded-sm border bg-muted/20 p-3 font-mono text-sm">
        {codes.map((code) => (
          <span key={code}>{code}</span>
        ))}
      </div>
      <div className="flex gap-2">
        <Button type="button" variant="outline" className="flex-1" onClick={copyCodes}>
          <Copy className="size-4" />
          Copy
        </Button>
        <Button type="button" variant="outline" className="flex-1" onClick={downloadCodes}>
          <Download className="size-4" />
          Download
        </Button>
      </div>
      <label className="flex items-center gap-2 text-sm" htmlFor="recovery-codes-confirmed">
        <Checkbox
          id="recovery-codes-confirmed"
          checked={confirmed}
          onCheckedChange={(value) => setConfirmed(value === true)}
        />
        I have saved my recovery codes.
      </label>
      <Button type="button" className="w-full" disabled={!confirmed} onClick={onContinue}>
        {actionLabel}
      </Button>
    </div>
  );
}
