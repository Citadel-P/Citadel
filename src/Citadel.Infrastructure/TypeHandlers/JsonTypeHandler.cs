using System.Data;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Dapper;

namespace Infrastructure.TypeHandlers;

public class JsonTypeHandler<T> : SqlMapper.TypeHandler<T>
{
    private readonly JsonTypeInfo<T> _typeInfo;

    public JsonTypeHandler(JsonTypeInfo<T> typeInfo)
    {
        _typeInfo = typeInfo ?? throw new ArgumentNullException(nameof(typeInfo));
    }

    public override void SetValue(IDbDataParameter parameter, T value)
    {
        parameter.Value = JsonSerializer.Serialize(value, _typeInfo);
    }

    public override T? Parse(object value)
    {
        if (value is not string json) return default;

        return JsonSerializer.Deserialize(json, _typeInfo);
    }
}
