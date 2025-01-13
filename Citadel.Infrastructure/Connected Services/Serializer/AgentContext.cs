using System;
using System.Collections.Generic;
using System.Data.SqlTypes;
using System.Linq;
using System.Text;
using System.Text.Json.Serialization;
using System.Threading.Tasks;

namespace Infrastructure.Connected_Services.Serializer;

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(string[]))]
[JsonSerializable(typeof(ContainerSummary))]
[JsonSerializable(typeof(Port))]
[JsonSerializable(typeof(HostConfig2))]
[JsonSerializable(typeof(NetworkSettings2))]
[JsonSerializable(typeof(MountPoint))]
[JsonSerializable(typeof(ICollection<MountPoint>))]
[JsonSerializable(typeof(EndpointSettings))]
[JsonSerializable(typeof(EndpointIPAMConfig))]
[JsonSerializable(typeof(ICollection<ContainerSummary>))]
[JsonSerializable(typeof(LogView))]
[JsonSerializable(typeof(ProblemDetails))]
[JsonSerializable(typeof(StreamLogsRequest))]
[JsonSerializable(typeof(SwarmInfoView))]
[JsonSerializable(typeof(SwarmPeerView))]
[JsonSerializable(typeof(SystemInfoView))]
internal partial class AgentContext : JsonSerializerContext
{
}