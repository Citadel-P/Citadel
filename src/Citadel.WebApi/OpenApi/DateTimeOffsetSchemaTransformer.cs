using Microsoft.AspNetCore.OpenApi;
using Microsoft.OpenApi;

namespace WebApi.OpenApi;

internal sealed class DateTimeOffsetSchemaTransformer : IOpenApiSchemaTransformer
{
    public Task TransformAsync(
        OpenApiSchema schema,
        OpenApiSchemaTransformerContext context,
        CancellationToken cancellationToken)
    {
        Apply(schema, context.JsonTypeInfo.Type);
        return Task.CompletedTask;
    }

    internal static void Apply(OpenApiSchema schema, Type type)
    {
        var underlyingType = Nullable.GetUnderlyingType(type);
        if (type != typeof(DateTimeOffset) && underlyingType != typeof(DateTimeOffset))
            return;

        schema.Type = JsonSchemaType.String;
        if (underlyingType is not null)
            schema.Type |= JsonSchemaType.Null;

        schema.Format = "date-time";
    }
}
