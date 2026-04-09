# See https://aka.ms/customizecontainer to learn how to customize your debug container and how Visual Studio uses this Dockerfile to build your images for faster debugging.

# These ARGs allow for swapping out the base used to make the final image when debugging from VS
ARG LAUNCHING_FROM_VS
# This sets the base image for final, but only if LAUNCHING_FROM_VS has been defined
ARG FINAL_BASE_IMAGE=${LAUNCHING_FROM_VS:+aotdebug}

# This stage is used when running from VS in fast mode (Default for Debug configuration)
FROM mcr.microsoft.com/dotnet/aspnet:10.0 AS base
USER $APP_UID
WORKDIR /app
EXPOSE 8080


# This stage is used to build the service project
FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
# Install clang/zlib1g-dev dependencies for publishing to native
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    clang zlib1g-dev
ARG BUILD_CONFIGURATION=Release
WORKDIR /src
COPY ["Directory.Packages.props", "."]
COPY ["src/Citadel.Contracts/Directory.Packages.props", "src/Citadel.Contracts/"]
COPY ["Directory.Build.props", "."]
COPY ["src/Citadel.Contracts/Directory.Build.props", "src/Citadel.Contracts/"]
COPY ["nuget.config", "."]
COPY ["src/Citadel.WebApi/Citadel.WebApi.csproj", "src/Citadel.WebApi/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting/Citadel.Hosting.csproj", "src/Citadel.Contracts/src/Citadel.Hosting/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting.Common/Citadel.Hosting.Common.csproj", "src/Citadel.Contracts/src/Citadel.Hosting.Common/"]
COPY ["src/Citadel.Application/Citadel.Application.csproj", "src/Citadel.Application/"]
COPY ["src/Citadel.Domain/Citadel.Domain.csproj", "src/Citadel.Domain/"]
COPY ["src/Citadel.Contracts/src/Citadel.SourceGen/Citadel.SourceGen.csproj", "src/Citadel.Contracts/src/Citadel.SourceGen/"]
COPY ["src/Citadel.Infrastructure/Citadel.Infrastructure.csproj", "src/Citadel.Infrastructure/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Citadel.Hosting.DockerClient.csproj", "src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/"]
RUN dotnet restore "./src/Citadel.WebApi/Citadel.WebApi.csproj"
COPY . .
WORKDIR "/src/src/Citadel.WebApi"
RUN dotnet build "./Citadel.WebApi.csproj" -c $BUILD_CONFIGURATION -o /app/build

# This stage is used to publish the service project to be copied to the final stage
FROM build AS publish
ARG BUILD_CONFIGURATION=Release
RUN dotnet publish "./Citadel.WebApi.csproj" -c $BUILD_CONFIGURATION -o /app/publish /p:UseAppHost=true

# This stage is used as the base for the final stage when launching from VS to support debugging in regular mode (Default when not using the Debug configuration)
FROM base AS aotdebug
USER root
# Install GDB to support native debugging
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    gdb
USER app

# This stage is used in production or when running from VS in regular mode (Default when not using the Debug configuration)
FROM ${FINAL_BASE_IMAGE:-mcr.microsoft.com/dotnet/runtime-deps:10.0} AS final
WORKDIR /app
EXPOSE 8080
COPY --from=publish /app/publish .
ENTRYPOINT ["./Citadel.WebApi"]