using System.Data;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Dapper;

namespace Infrastructure.DapperHandlers;

public class JsonTypeHandler<T> : SqlMapper.TypeHandler<T>
{
    private readonly JsonSerializerOptions? _options;
    private readonly JsonTypeInfo<T>? _typeInfo;

    public JsonTypeHandler(JsonSerializerOptions? options = null)
    {
        _options = options;
    }

    public JsonTypeHandler(JsonTypeInfo<T> typeInfo)
    {
        _typeInfo = typeInfo;
    }

    public override void SetValue(IDbDataParameter parameter, T value)
    {
        parameter.Value = _typeInfo is not null
            ? JsonSerializer.Serialize(value, _typeInfo)
            : JsonSerializer.Serialize(value, _options);
    }

    public override T? Parse(object value)
    {
        if (value is not string json) return default;

        return _typeInfo is not null
            ? JsonSerializer.Deserialize(json, _typeInfo)
            : JsonSerializer.Deserialize<T>(json, _options);
    }
}