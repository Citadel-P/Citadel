//using System.Text.Json;
//using Infrastructure;
//using Infrastructure.Entities;
//using Mediator;
//using Microsoft.AspNetCore.Authorization;
//using Microsoft.AspNetCore.Mvc;
//using Hosting.Extensions;
//using WebApi.Controllers.V1.Resources.Registries;

//namespace WebApi.Controllers.V1;

///// <summary>
///// The Registries controller
///// </summary>
//[Authorize]
//[ApiController]
//[Produces("application/json")]
//[Route("api/v{version:apiVersion}/[controller]")]
//public sealed class RegistriesController(IMediator mediator) : ControllerBase
//{
//    /// <summary>
//    /// Create a registry,
//    /// </summary>
//    /// <remarks>
//    /// Please note that a discriminator should be provided in the <see cref="CreateRegistryRequest.Configuration"/> property,
//    /// this discriminator is based on <see cref="RegistryDiscriminator"/> enum
//    ///
//    /// Sample request:
//    ///
//    ///     POST /Registries
//    ///     {
//    ///         "name": "azure repo 1",
//    ///         "url": "https://azure.com",
//    ///         "discriminator": "Azure",
//    ///         "configuration": {
//    ///          "$type" : 1,
//    ///          "userName" : "JohnDoe",
//    ///          "Password": "124532sdsdsq89zefs"
//    ///         }
//    ///     }
//    ///
//    /// </remarks>
//    //[HttpPost]
//    //[ProducesResponseType(StatusCodes.Status200OK)]
//    //[ProducesResponseType(typeof(string), StatusCodes.Status409Conflict)]
//    //[ProducesResponseType(typeof(string), StatusCodes.Status400BadRequest)]
//    //public async Task<ActionResult<RegistryResponse>> Create(CreateRegistryRequest request, CancellationToken cancellationToken)
//    //{
//    //    var response = await mediator.Send(request.ToCommand(), cancellationToken);
//    //    return this.HandleResult(response, RegistryResponse.Map);
//    //}

//    /// <summary>
//    /// Get all registries
//    /// </summary>
//    /// <param name="cancellationToken"></param>
//    /// <returns></returns>
//    //[HttpGet]
//    //[ProducesResponseType(StatusCodes.Status200OK)]
//    //public async Task<ActionResult<IEnumerable<RegistryResponse>>> GetAll(CancellationToken cancellationToken)
//    //{
//    //    var response = await mediator.Send(new GetAllRegistries(), cancellationToken);
//    //    return Deserialize(response).ToList();
//    //}

//    private static IEnumerable<RegistryResponse> Deserialize(IEnumerable<Registry> registries)
//    {
//        List<RegistryResponse> result = [];

//        // Deserialize
//        foreach (var registry in registries)
//        {
//            IRegistryConfiguration configuration = null;
//            switch (registry.Discriminator)
//            {
//                case RegistryDiscriminator.Azure:
//                    configuration = JsonSerializer.Deserialize<AzureRegistry>(registry.Configuration);
//                    break;

//                case RegistryDiscriminator.AWS:
//                    configuration = JsonSerializer.Deserialize<AWSRegistry>(registry.Configuration);
//                    break;

//                case RegistryDiscriminator.DockerHub:
//                    configuration = JsonSerializer.Deserialize<DockerHubRegistry>(registry.Configuration);
//                    break;

//                case RegistryDiscriminator.Gitlab:
//                    configuration = JsonSerializer.Deserialize<GitlabRegistry>(registry.Configuration);
//                    break;

//                case RegistryDiscriminator.Custom:
//                    configuration = JsonSerializer.Deserialize<CustomRegistry>(registry.Configuration);
//                    break;
//            }
//            result.Add(new RegistryResponse(registry.Id, registry.Name, registry.Url, registry.Created, configuration));
//        }
//        return result;
//    }
//}