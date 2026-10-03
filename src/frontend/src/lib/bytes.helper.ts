export type ByteUnit = 'B' | 'kB' | 'KB' | 'MB' | 'GB' | 'TB';

const formats: { [key: string]: { max: number; prev?: ByteUnit } } = {
  B: { max: 1024 },
  kB: { max: Math.pow(1024, 2), prev: 'B' },
  KB: { max: Math.pow(1024, 2), prev: 'B' }, // Backward compatible
  MB: { max: Math.pow(1024, 3), prev: 'kB' },
  GB: { max: Math.pow(1024, 4), prev: 'MB' },
  TB: { max: Number.MAX_SAFE_INTEGER, prev: 'GB' },
};

/**
 * Transforms a byte value into a human-readable format (e.g., KB, MB, GB).
 *
 * @param input The numeric value to transform. If not a finite number, it is returned as is.
 * @param decimal The number of decimal places to round the result to. Defaults to 0.
 * @param from The unit of the input value. Defaults to 'B' (bytes).
 * @param to The desired output unit. If not provided, the function will automatically select the best unit.
 * @returns The formatted string with the transformed value and unit, or the original input if it's not a valid number.
 */
export function byteTransform(input: any, decimal: number = 0, from: ByteUnit = 'B', to?: ByteUnit): any {
  if (!(isNumberFinite(input) && isNumberFinite(decimal) && isInteger(decimal) && isPositive(decimal))) {
    return input;
  }

  // Convert the input value to bytes first.
  let bytes = input;
  let unit = from;
  while (unit !== 'B') {
    bytes *= 1024;
    unit = formats[unit].prev!;
  }

  // If a specific output unit is requested, convert to that unit.
  if (to) {
    const format = formats[to];
    const result = toDecimal(calculateResult(format, bytes), decimal);
    return formatResult(result, to);
  }

  // If no output unit is specified, find the most appropriate unit.
  for (const key in formats) {
    if (Object.prototype.hasOwnProperty.call(formats, key)) {
      const format = formats[key];
      if (bytes < format.max) {
        const result = toDecimal(calculateResult(format, bytes), decimal);
        return formatResult(result, key);
      }
    }
  }
}

/**
 * Formats the result as a string with the value and unit.
 * @param result The numeric value.
 * @param unit The unit string.
 * @returns The formatted string.
 */
function formatResult(result: number, unit: string): string {
  return `${result} ${unit}`;
}

/**
 * Calculates the value in the target unit.
 * @param format The format object for the target unit.
 * @param bytes The value in bytes.
 * @returns The calculated value.
 */
function calculateResult(format: { max: number; prev?: ByteUnit }, bytes: number): number {
  const prev = format.prev ? formats[format.prev] : undefined;
  return prev ? bytes / prev.max : bytes;
}

/**
 * Checks if a value is a number.
 * @param value The value to check.
 * @returns True if the value is a number, false otherwise.
 */
function isNumber(value: any): value is number {
  return typeof value === 'number';
}

/**
 * Checks if a value is a finite number.
 * @param value The value to check.
 * @returns True if the value is a finite number, false otherwise.
 */
function isNumberFinite(value: any): value is number {
  return isNumber(value) && isFinite(value);
}

/**
 * Checks if a value is a positive number (including zero).
 * @param value The value to check.
 * @returns True if the value is greater than or equal to 0.
 */
function isPositive(value: number): boolean {
  return value >= 0;
}

/**
 * Checks if a value is an integer.
 * @param value The value to check.
 * @returns True if the value is an integer, false otherwise.
 */
function isInteger(value: number): boolean {
  return value % 1 === 0;
}

/**
 * Rounds a number to a specified number of decimal places.
 * @param value The number to round.
 * @param decimal The number of decimal places.
 * @returns The rounded number.
 */
function toDecimal(value: number, decimal: number): number {
  return Math.round(value * Math.pow(10, decimal)) / Math.pow(10, decimal);
}