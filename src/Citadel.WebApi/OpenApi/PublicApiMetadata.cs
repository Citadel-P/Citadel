using Microsoft.AspNetCore.Mvc.ApiExplorer;

namespace WebApi.OpenApi;

internal sealed class PublicApiMetadata
{
    public static PublicApiMetadata Instance { get; } = new();

    private PublicApiMetadata() { }
}

internal static class PublicApiEndpointConventionBuilderExtensions
{
    public static TBuilder WithPublicApi<TBuilder>(this TBuilder builder)
        where TBuilder : IEndpointConventionBuilder
    {
        builder.WithMetadata(PublicApiMetadata.Instance);
        return builder;
    }

    public static bool IsPublicApi(this ApiDescription description)
        => description.ActionDescriptor.EndpointMetadata
            .OfType<PublicApiMetadata>()
            .Any();
}
