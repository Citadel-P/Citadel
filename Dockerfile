FROM mcr.microsoft.com/dotnet/aspnet:9.0 AS base
WORKDIR /app
EXPOSE 8000
EXPOSE 8001

ENV DOTNET_GCServer=1 \
    DOTNET_GCHighMemoryPercent=20 \
    DOTNET_GCHeapCount=2 \
    DOTNET_TC_OptimizeForContainer=1 \
    DOTNET_GCHeapHardLimitPercent=75 \
    DOTNET_ThreadPool_ForceMinWorkerThreads=4

FROM mcr.microsoft.com/dotnet/sdk:9.0 AS build
ARG BUILD_CONFIGURATION=Release
WORKDIR /src

COPY ["Directory.Build.props", "."]
COPY ["nuget.config", "."]
COPY ["Citadel.WebApi/Citadel.WebApi.csproj", "Citadel.WebApi/"]
COPY ["Citadel.Application/Citadel.Application.csproj", "Citadel.Application/"]
COPY ["Citadel.Infrastructure/Citadel.Infrastructure.csproj", "Citadel.Infrastructure/"]
COPY ["Citadel.Contracts/Citadel.Hosting/Citadel.Hosting.csproj", "Citadel.Contracts/Citadel.Hosting/"]
COPY ["Citadel.Contracts/Citadel.Hosting.Common/Citadel.Hosting.Common.csproj", "Citadel.Contracts/Citadel.Hosting.Common/"]
COPY ["Citadel.Contracts/Citadel.SourceGen/Citadel.SourceGen.csproj", "Citadel.Contracts/Citadel.SourceGen/"]

RUN dotnet restore "./Citadel.WebApi/Citadel.WebApi.csproj"

COPY . .
WORKDIR "/src/Citadel.WebApi"
RUN dotnet build "./Citadel.WebApi.csproj" -c $BUILD_CONFIGURATION -o /app/build

FROM build AS publish
ARG BUILD_CONFIGURATION=Release
RUN dotnet publish "./Citadel.WebApi.csproj" -c $BUILD_CONFIGURATION -o /app/publish /p:UseAppHost=true 

FROM base AS final
WORKDIR /app
COPY --from=publish /app/publish .
ENTRYPOINT ["dotnet", "Citadel.WebApi.dll"]