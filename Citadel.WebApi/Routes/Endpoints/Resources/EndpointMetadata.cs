namespace WebApi.Routes.Endpoints.Resources;

public sealed record EndpointMetadata(bool? CanEdit = false, bool? CanDelete = false);
