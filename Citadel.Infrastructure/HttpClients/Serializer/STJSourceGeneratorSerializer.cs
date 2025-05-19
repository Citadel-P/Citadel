using System.Net.Http.Json;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Refit;

namespace Infrastructure.HttpClients.Serializer;

public sealed class STJSourceGeneratorSerializer(JsonSerializerOptions jsonSerializerOptions) : IHttpContentSerializer
{
    public STJSourceGeneratorSerializer() : this(HttpClientsContext.JsonSerializerOptions) { }

    public Task<T?> FromHttpContentAsync<T>(HttpContent content, CancellationToken cancellationToken = default)
        => content.ReadFromJsonAsync((JsonTypeInfo<T>)jsonSerializerOptions.GetTypeInfo(typeof(T)), cancellationToken);

    public HttpContent ToHttpContent<T>(T item)
        => JsonContent.Create(item, (JsonTypeInfo<T>)jsonSerializerOptions.GetTypeInfo(typeof(T)));

    public string? GetFieldNameForProperty(PropertyInfo propertyInfo) =>
        propertyInfo?.GetCustomAttribute<JsonPropertyNameAttribute>(true)?.Name;

}