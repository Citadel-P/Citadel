import { SelectField } from '@/components/custom/common';

type TimezoneOption = {
  value: string;
  label: string;
};

type TimezoneSeed = TimezoneOption & {
  offsetMinutes: number;
};

const TIMEZONE_SEED: TimezoneSeed[] = [
  { value: 'UTC', label: 'Coordinated Universal Time (UTC +/-00:00)', offsetMinutes: 0 },
  { value: 'Etc/GMT', label: 'Greenwich Mean Time (UTC +/-00:00)', offsetMinutes: 0 },
  { value: 'Europe/Paris', label: 'Central European Time (UTC +01:00)', offsetMinutes: 60 },
  { value: 'Africa/Lagos', label: 'West Africa Time (UTC +01:00)', offsetMinutes: 60 },
  { value: 'Europe/Athens', label: 'Eastern European Time (UTC +02:00)', offsetMinutes: 120 },
  { value: 'Africa/Harare', label: 'Central Africa Time (UTC +02:00)', offsetMinutes: 120 },
  { value: 'Africa/Johannesburg', label: 'South Africa Standard Time (UTC +02:00)', offsetMinutes: 120 },
  { value: 'Europe/Moscow', label: 'Moscow Time (UTC +03:00)', offsetMinutes: 180 },
  { value: 'Asia/Dubai', label: 'Gulf Standard Time (UTC +04:00)', offsetMinutes: 240 },
  { value: 'Asia/Karachi', label: 'Pakistan Time (UTC +05:00)', offsetMinutes: 300 },
  { value: 'Asia/Kolkata', label: 'India Standard Time (UTC +05:30)', offsetMinutes: 330 },
  { value: 'Asia/Bangkok', label: 'Indochina Time (UTC +07:00)', offsetMinutes: 420 },
  { value: 'Asia/Shanghai', label: 'China Standard Time (UTC +08:00)', offsetMinutes: 480 },
  { value: 'Australia/Perth', label: 'Australian Western Standard Time (UTC +08:00)', offsetMinutes: 480 },
  { value: 'Asia/Tokyo', label: 'Japan Standard Time (UTC +09:00)', offsetMinutes: 540 },
  { value: 'Asia/Seoul', label: 'Korea Standard Time (UTC +09:00)', offsetMinutes: 540 },
  { value: 'Australia/Adelaide', label: 'Australian Central Standard Time (UTC +09:30)', offsetMinutes: 570 },
  { value: 'Australia/Sydney', label: 'Australian Eastern Standard Time (UTC +10:00)', offsetMinutes: 600 },
  { value: 'Pacific/Auckland', label: 'New Zealand Standard Time (UTC +12:00)', offsetMinutes: 720 },
  { value: 'America/New_York', label: 'Eastern Standard Time (UTC -05:00)', offsetMinutes: -300 },
  { value: 'America/Chicago', label: 'Central Standard Time (UTC -06:00)', offsetMinutes: -360 },
  { value: 'America/Denver', label: 'Mountain Standard Time (UTC -07:00)', offsetMinutes: -420 },
  { value: 'America/Los_Angeles', label: 'Pacific Standard Time (UTC -08:00)', offsetMinutes: -480 },
  { value: 'America/Anchorage', label: 'Alaska Standard Time (UTC -09:00)', offsetMinutes: -540 },
  { value: 'Pacific/Honolulu', label: 'Hawaii Standard Time (UTC -10:00)', offsetMinutes: -600 },
];

export const TIMEZONE_OPTIONS = TIMEZONE_SEED.slice()
  .sort((a, b) => {
    const aKey = a.offsetMinutes >= 0 ? a.offsetMinutes : 10000 + Math.abs(a.offsetMinutes);
    const bKey = b.offsetMinutes >= 0 ? b.offsetMinutes : 10000 + Math.abs(b.offsetMinutes);
    return aKey - bKey || a.label.localeCompare(b.label);
  })
  .map(({ offsetMinutes: _, ...timezone }) => timezone);

export const getDefaultTimezone = () => {
  return TIMEZONE_OPTIONS[0]?.value ?? 'UTC';
};

export const getBrowserTimezone = () => {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || getDefaultTimezone();
  } catch {
    return getDefaultTimezone();
  }
};

export function TimezoneSelectField({
  value,
  onChange,
  disabled,
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}) {
  return (
    <SelectField
      value={value}
      onChange={onChange}
      options={TIMEZONE_OPTIONS}
      placeholder="Select timezone"
      allLabel="Timezone"
      selectableLabel={false}
      disabled={disabled}
      className={className}
    />
  );
}
