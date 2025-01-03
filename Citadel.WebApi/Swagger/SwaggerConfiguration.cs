using Swashbuckle.AspNetCore.SwaggerUI;

namespace WebApi.Swagger;

public static class SwaggerConfiguration
{
    public const string PublicApiV1 = "public-v1";
    public const string InternalApiV1 = "internal-v1";

    public static void AddCustomSwaggerUIOptions(this SwaggerUIOptions options, bool isDevelopment)
    {
        if (isDevelopment)
        {
            options.EnableDeepLinking();
            options.DisplayOperationId();
            options.DisplayRequestDuration();
            options.EnableTryItOutByDefault();
            options.EnablePersistAuthorization();
            options.InjectStylesheet("../../swagger-ui/custom.css");
        }

        options.SwaggerEndpoint($"/openapi/{PublicApiV1}.json", PublicApiV1);
        options.SwaggerEndpoint($"/openapi/{InternalApiV1}.json", InternalApiV1);
    }
}
