using Domain;
using Microsoft.AspNetCore.OpenApi;
using Microsoft.OpenApi;
using System.Text.Json.Nodes;

namespace WebApi.OpenApi;

internal sealed class KnownEnumSchemaDocumentTransformer : IOpenApiDocumentTransformer
{
    public Task TransformAsync(OpenApiDocument document, OpenApiDocumentTransformerContext context, CancellationToken cancellationToken)
    {
        SetEnumSchema<BackupRunStatus>(document);
        return Task.CompletedTask;
    }

    private static void SetEnumSchema<TEnum>(OpenApiDocument document)
        where TEnum : struct, Enum
    {
        if (document.Components?.Schemas is null ||
            !document.Components.Schemas.TryGetValue(typeof(TEnum).Name, out var schema) ||
            schema is not OpenApiSchema mutableSchema)
        {
            return;
        }

        mutableSchema.Format = null;
        mutableSchema.Type = JsonSchemaType.String;
        mutableSchema.Enum?.Clear();
        mutableSchema.Enum ??= [];

        foreach (var name in Enum.GetNames<TEnum>())
        {
            mutableSchema.Enum.Add(JsonValue.Create(name));
        }
    }
}
