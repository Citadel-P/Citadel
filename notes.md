
* Clone the repo, and it's submodule: `git submodule update --init` 

* To generate a database migration:
   Build and run the `Citadel.Infrastructure.MigrationTool` console application using `dotnet run --project Citadel.Infrastructure.MigrationTool`.
   This tool generates both schema and seed SQL migration files for the database, outputting them to the `Citadel.Infrastructure.Scripts` folder.
* Build the image: `docker build --no-cache --platform=linux/amd64 -t citadel.v1 .`
	* Run the image:  `docker run -d -p 8000:8000 -p 8001:8001 -v "citadel_data:/app/data" -v "/var/run/docker.sock:/var/run/docker.sock" --name citadel.v1 citadel.v1`