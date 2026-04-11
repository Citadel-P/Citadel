using System.Collections.Concurrent;
using System.Data;

namespace Infrastructure.TypeHandlers;

internal static class FormattingExtensions
{
    internal static class EnumFormatter<TEnum> where TEnum : struct, Enum
    {
        private static readonly ConcurrentDictionary<TEnum, string> nameCache = new();
        public static string GetValue(TEnum value)
        {
           return nameCache.GetOrAdd(value, static e => Enum.GetName(e) ?? e.ToString());
        }
    }
}
