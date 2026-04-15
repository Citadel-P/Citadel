FROM node:24-alpine AS build
WORKDIR /src/Citadel.FrontEnd

COPY ["src/Citadel.FrontEnd/package.json", "src/Citadel.FrontEnd/package-lock.json", "./"]
RUN npm ci

COPY ["src/Citadel.FrontEnd/", "./"]
RUN npm run build:prod

FROM alpine:latest
WORKDIR /dist
COPY --from=build /src/Citadel.FrontEnd/dist .