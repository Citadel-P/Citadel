using Application.Features.Images.Commands;

namespace WebApi.Routes.Endpoints.Resources.Images;

public sealed record DeleteImagesRequest(Guid PlatformId, string[] Ids, bool Force = false, bool NoPrune = false)
{
    internal DeleteImages ToCommand() => new(PlatformId, Ids, Force, NoPrune);
}
