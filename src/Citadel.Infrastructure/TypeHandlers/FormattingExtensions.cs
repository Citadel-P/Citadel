using System.Collections.Concurrent;
using System.Data;

namespace Infrastructure.TypeHandlers;

internal static class FormattingExtensions
{
    public static IEnumerable<string> Format(this IEnumerable<Guid> value)
        => value.Select(Format);

    public static string Format(this Guid value)
    {
        Span<char> buffer = stackalloc char[36]; // 36 is max for "D" format
        return value.TryFormat(buffer, out int charsWritten, "D")
            ? new string(buffer[..charsWritten])
            : value.ToString("D");
    }

    internal static class EnumFormatter<TEnum> where TEnum : struct, Enum
    {
        private static readonly ConcurrentDictionary<TEnum, string> nameCache = new();
        public static string GetValue(TEnum value)
        {
           return nameCache.GetOrAdd(value, static e => Enum.GetName(e) ?? e.ToString());
        }
    }
}
