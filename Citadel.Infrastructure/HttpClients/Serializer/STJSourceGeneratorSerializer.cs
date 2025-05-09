using System.Net.Http.Json;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;
using Refit;

namespace Infrastructure.HttpClients.Serializer;

public sealed class STJSourceGeneratorSerializer(JsonSerializerOptions jsonSerializerOptions) : IHttpContentSerializer
{
    public static readonly JsonSerializerOptions DefaultOptions = CreateDefaultOptions();

    public STJSourceGeneratorSerializer() : this(DefaultOptions) { }

    public Task<T?> FromHttpContentAsync<T>(HttpContent content, CancellationToken cancellationToken = default)
    => content.ReadFromJsonAsync<T>(jsonSerializerOptions, cancellationToken);

    public HttpContent ToHttpContent<T>(T item) =>
        JsonContent.Create(item, options: jsonSerializerOptions);

    public string GetFieldNameForProperty(PropertyInfo propertyInfo) =>
        propertyInfo?.GetCustomAttribute<JsonPropertyNameAttribute>(true)?.Name;

    private static JsonSerializerOptions CreateDefaultOptions()
    {
        var options = new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = false,
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
            NumberHandling = JsonNumberHandling.AllowNamedFloatingPointLiterals,
        };

        HttpClientsContext.RegisterContexts(options.TypeInfoResolverChain);

        options.Converters.Add(new JsonStringEnumConverter());

        return options;
    }
}