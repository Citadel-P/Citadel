using System.Net.Http.Json;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;
using Refit;

namespace Infrastructure.Connected_Services.Serializer;

public sealed class STJSourceGeneratorSerializer(JsonSerializerOptions jsonSerializerOptions) : IHttpContentSerializer
{
    public STJSourceGeneratorSerializer() : this(GetDefaultJsonSerializerOptions())
    {
    }

    public async Task<T?> FromHttpContentAsync<T>(HttpContent content, CancellationToken cancellationToken = default)
        => await content.ReadFromJsonAsync<T>(jsonSerializerOptions, cancellationToken);

    public HttpContent ToHttpContent<T>(T item)
        => JsonContent.Create(item, options: jsonSerializerOptions);

    public string GetFieldNameForProperty(PropertyInfo propertyInfo)
    {
        _ = propertyInfo ?? throw new ArgumentNullException(nameof(propertyInfo));

        return propertyInfo.GetCustomAttributes<JsonPropertyNameAttribute>(true)
                   .Select(a => a.Name)
                   .FirstOrDefault();
    }

    /// <summary>
    /// Creates new <see cref="JsonSerializerOptions"/> and fills it with custom parameters
    /// </summary>
    private static JsonSerializerOptions GetDefaultJsonSerializerOptions()
    {
        var serializerOptions = new JsonSerializerOptions()
        {
            PropertyNameCaseInsensitive = false,
            PropertyNamingPolicy = null,
        };

        serializerOptions.TypeInfoResolverChain.Add(AgentContext.Default);

        serializerOptions.Converters.Add(new ObjectToInferredTypesConverter());
        serializerOptions.Converters.Add(new JsonStringEnumConverter());
        serializerOptions.NumberHandling = JsonNumberHandling.AllowNamedFloatingPointLiterals;
        return serializerOptions;
    }
}