
* Clone the repo, and it's submodule: `git submodule update --init` 
* In Visual Studio set the docker-compose project as the startup project.
* You are set, have fun

* The WebApi project has a folder called `data`, this folder is mapped to a named volume in docker-compose
  * When deploying the Docker image, the named volume should be provided

* To generate a db migration
  * Open the infrastructure project in a terminal.
  * Run `dotnet ef migrations add migration0001` - update ef if needed (`dotnet tool update --global dotnet-ef`)
  * Check the generated files for any errors
  * Generate script `dotnet ef migrations script -o "Migrations/script0001 - Init Db.sql"`