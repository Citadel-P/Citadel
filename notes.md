
* Clone the repo, and it's submodule: `git submodule update --init` 

* To generate a db migration
  * Open the infrastructure project in a terminal.
  * Run `dotnet ef migrations add migration0001 -o .\Migrations\` - update ef if needed (`dotnet tool update --global dotnet-ef`)
  * Check the generated files for any errors
  * Generate script `dotnet ef migrations script -o "Migrations/script.sql"`

*  Build the image: `docker build --no-cache --platform=linux/amd64 -t citadel.v1 .`
	* Run the image:  `docker run -d -p 8000:8000 -p 8001:8001 -v "citadel_data:/app/data" -v "/var/run/docker.sock:/var/run/docker.sock" --name citadel.v1 citadel.v1`