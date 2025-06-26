FROM mcr.microsoft.com/dotnet/aspnet:9.0 AS base
WORKDIR /app
EXPOSE 8000
EXPOSE 8001

ENV \
    DOTNET_GCServer=1 \
    DOTNET_System_GC_RetainVM=0 \
    DOTNET_TC_OptimizeForContainer=1 \
    DOTNET_GCHeapHardLimitPercent=75

FROM mcr.microsoft.com/dotnet/sdk:9.0 AS build
WORKDIR /src

COPY ["Directory.Build.props", "."]
COPY ["nuget.config", "."]
COPY ["Citadel.WebApi/Citadel.WebApi.csproj", "Citadel.WebApi/"]
COPY ["Citadel.Domain/Citadel.Domain.csproj", "Citadel.Domain/"]
COPY ["Citadel.Application/Citadel.Application.csproj", "Citadel.Application/"]
COPY ["Citadel.Infrastructure/Citadel.Infrastructure.csproj", "Citadel.Infrastructure/"]
COPY ["Citadel.Contracts/Citadel.Hosting/Citadel.Hosting.csproj", "Citadel.Contracts/Citadel.Hosting/"]
COPY ["Citadel.Contracts/Citadel.SourceGen/Citadel.SourceGen.csproj", "Citadel.Contracts/Citadel.SourceGen/"]
COPY ["Citadel.Contracts/Citadel.Hosting.Common/Citadel.Hosting.Common.csproj", "Citadel.Contracts/Citadel.Hosting.Common/"]
COPY ["Citadel.Contracts/Citadel.Hosting.DockerClient/Citadel.Hosting.DockerClient.csproj", "Citadel.Contracts/Citadel.Hosting.DockerClient/"]

RUN dotnet restore "./Citadel.WebApi/Citadel.WebApi.csproj"

COPY . .
WORKDIR "/src/Citadel.WebApi"
RUN dotnet build "./Citadel.WebApi.csproj" -c Release -o /app/build

FROM build AS publish
RUN dotnet publish "./Citadel.WebApi.csproj" -c Release -o /app/publish

FROM base AS final
WORKDIR /app

COPY --from=publish /app/publish .

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

ENTRYPOINT ["dotnet", "Citadel.WebApi.dll"]