using Microsoft.AspNetCore.OpenApi;
using Microsoft.OpenApi;

namespace WebApi.OpenApi;

internal sealed class PublicApiDocumentTransformer : IOpenApiDocumentTransformer
{
    public Task TransformAsync(
        OpenApiDocument document,
        OpenApiDocumentTransformerContext context,
        CancellationToken cancellationToken)
    {
        document.Info ??= new OpenApiInfo();
        document.Info.Title = "Citadel API Preview";
        document.Info.Description =
            "The explicitly supported Citadel HTTP API subset. This contract remains a preview until a compatibility and deprecation policy is published.";
        document.Servers =
        [
            new OpenApiServer
            {
                Url = "https://citadel.example.com",
                Description = "Replace this reserved example hostname with the URL of your Citadel installation."
            }
        ];

        MergeEquivalentPath(
            document,
            "/api/v1/deployments/{id}",
            "/api/v1/deployments/{deploymentId}",
            "id",
            "deploymentId");
        MergeEquivalentPath(
            document,
            "/api/v1/stacks/{id}",
            "/api/v1/stacks/{stackId}",
            "id",
            "stackId");

        if (document.Tags is not null)
        {
            foreach (var tag in document.Tags.OfType<OpenApiTag>())
            {
                tag.Description ??= $"Operations for Citadel {tag.Name} resources.";
            }
        }

        foreach (var operation in document.Paths.Values.SelectMany(path => path.Operations ?? []))
        {
            operation.Value.Responses ??= new OpenApiResponses();
            operation.Value.Responses.TryAdd("401", new OpenApiResponse
            {
                Description = "Authentication is required."
            });
            operation.Value.Responses.TryAdd("403", new OpenApiResponse
            {
                Description = "The authenticated actor is not authorized for this operation."
            });

            operation.Value.Security ??= [];
            if (operation.Value.Security.Count == 0)
            {
                operation.Value.Security.Add(new OpenApiSecurityRequirement
                {
                    {
                        new OpenApiSecuritySchemeReference("CitadelBearer", document), []
                    }
                });
            }
        }

        return Task.CompletedTask;
    }

    private static void MergeEquivalentPath(
        OpenApiDocument document,
        string sourcePath,
        string destinationPath,
        string sourceParameter,
        string destinationParameter)
    {
        if (!document.Paths.TryGetValue(sourcePath, out var source)
            || !document.Paths.TryGetValue(destinationPath, out var destination)
            || source.Operations is null
            || destination.Operations is null)
        {
            return;
        }

        foreach (var operation in source.Operations)
        {
            if (operation.Value.Parameters is not null)
            {
                foreach (var parameter in operation.Value.Parameters.OfType<OpenApiParameter>())
                {
                    if (parameter.In == ParameterLocation.Path
                        && string.Equals(parameter.Name, sourceParameter, StringComparison.Ordinal))
                    {
                        parameter.Name = destinationParameter;
                    }
                }
            }

            destination.Operations.TryAdd(operation.Key, operation.Value);
        }

        document.Paths.Remove(sourcePath);
    }
}
