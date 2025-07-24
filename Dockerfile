FROM mcr.microsoft.com/dotnet/aspnet:10.0-preview-alpine AS base
WORKDIR /app
EXPOSE 8000
EXPOSE 8001

ENV \
    DOTNET_RUNNING_IN_CONTAINER=true \
    DOTNET_GCServer=1 \
    DOTNET_System_GC_RetainVM=0 \
    DOTNET_TC_OptimizeForContainer=1

FROM mcr.microsoft.com/dotnet/sdk:10.0-preview-alpine AS build
RUN apk add --no-cache libc6-compat

WORKDIR /src

COPY ["nuget.config", "."]
COPY ["Directory.Build.props", "."]
COPY ["Directory.Packages.props", "."]

COPY ["src/Citadel.WebApi/Citadel.WebApi.csproj", "Citadel.WebApi/"]
COPY ["src/Citadel.Domain/Citadel.Domain.csproj", "Citadel.Domain/"]
COPY ["src/Citadel.Application/Citadel.Application.csproj", "Citadel.Application/"]
COPY ["src/Citadel.Infrastructure/Citadel.Infrastructure.csproj", "Citadel.Infrastructure/"]

COPY ["src/Citadel.Contracts/Directory.Build.props", "Citadel.Contracts/"]
COPY ["src/Citadel.Contracts/Directory.Packages.props", "Citadel.Contracts/"]

COPY ["src/Citadel.Contracts/Citadel.Hosting/Citadel.Hosting.csproj", "Citadel.Contracts/Citadel.Hosting/"]
COPY ["src/Citadel.Contracts/Citadel.SourceGen/Citadel.SourceGen.csproj", "Citadel.Contracts/Citadel.SourceGen/"]
COPY ["src/Citadel.Contracts/Citadel.Hosting.Common/Citadel.Hosting.Common.csproj", "Citadel.Contracts/Citadel.Hosting.Common/"]
COPY ["src/Citadel.Contracts/Citadel.Hosting.DockerClient/Citadel.Hosting.DockerClient.csproj", "Citadel.Contracts/Citadel.Hosting.DockerClient/"]

RUN dotnet restore "./Citadel.WebApi/Citadel.WebApi.csproj"

COPY src/ .
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