using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities;

namespace Application.Services.SignalR;

internal interface IImageStreamManager
{
    Task SendImagesInfo(Guid platformId, IEnumerable<Image> images);
}

internal class ImageStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IImageStreamManager
{
    public Task SendImagesInfo(Guid platformId, IEnumerable<Image> images)
    {
        //throw new NotImplementedException();
        return Task.CompletedTask;
    }
}

