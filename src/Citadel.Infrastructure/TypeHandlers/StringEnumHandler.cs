using System.Collections.Concurrent;
using System.Data;
using Dapper;

namespace Infrastructure.TypeHandlers;

public sealed class StringEnumHandler<TEnum> : SqlMapper.TypeHandler<TEnum>
    where TEnum : struct, Enum
{
    private static readonly ConcurrentDictionary<TEnum, string> nameCache = new();

    public override void SetValue(IDbDataParameter parameter, TEnum value)
    {
        parameter.Value = nameCache.GetOrAdd(value, static e => Enum.GetName(e) ?? e.ToString());
    }

    public override TEnum Parse(object value)
    {
        if (value is string str && Enum.TryParse<TEnum>(str, ignoreCase: true, out var result))
            return result;

        throw new InvalidCastException($"Cannot convert value '{value}' to enum {typeof(TEnum).Name}");
    }
}