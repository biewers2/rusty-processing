# file-processing

This project consists of a set of libraries and applications used to "process" file types. The term "process" in this
context means the following:
* Text extraction
* Metadata extraction
* File -> PDF conversion ("rendering")
* Embedded/Attached file extraction

### Project Structure

| Crate             | Description                                                                                                   |
|-------------------|---------------------------------------------------------------------------------------------------------------|
| `processing`      | Core library used by all application binaries<br> This is where per-filetype processing implementation exists |
| `identify`        | File's MIME type identification and de-duplication checksum calculations                                      |
| `services`        | Common services used by other libraries and applications                                                      |
| `cli`             | Command-line interface for processing files, primarily used for debugging                                     |
| `temporal-worker` | Worker implementing Temporal IO activities for processing files                                               |

### Build & Run

Being a Rust project, `cargo` is the build tool used. See https://github.com/rust-lang/cargo for how to use.

#### Running the Worker in a Container

The `temporal-worker` application runs as its own service. In order to run the Temporal worker in a container, it
must be able to connect to a Temporal server. The Temporal server and how to start up its containers can be found here:
https://github.com/temporalio/docker-compose

Once the Temporal server and its auxiliary containers are running, set up environment variables needed by the worker.
These can be placed in a `.env` file (recommended) or set in the shell environment where the following commands are run.

| Variable        | Value         | Description                                                                                                                             |
|-----------------|---------------|-----------------------------------------------------------------------------------------------------------------------------------------|
| `TEMPORAL_HOST` | `temporal`    | Host of the temporal server to connect to; the value is defined by the name of the Docker Compose service running                       |
| `TEMPORAL_PORT` | `7233`        | Port of the temporal server to connect to; the value is defined by the established port in the Temporal server's Docker Compose file    |
| `TIKA_HOST`     | `apache-tika` | Host of the Apache Tika server to connect to; the value is defined by the name of the Docker Compose service running                    |
| `TIKA_PORT`     | `9998`        | Port of the Apache Tika server to connect to; the value is defined by the established port in the Temporal server's Docker Compose file |

Then run the following commands:

```bash
docker build -t rusty-processing-worker . # To build the Docker image containing the worker binary
docker compose up -d # To start the worker container
```

_Note_: The container defined by this project's Docker Compose file binds the source code to the `/code` directory
in the container. This allows the developer to run tests and the CLI in a controlled environment.

### Notes, Context, and Standards on Processing Implementations

Sourced from the [EDRM Production Model](https://edrm.net/resources/frameworks-and-standards/edrm-model/production/), section 3.3 "Fielded Data".

| File Elements | Metadata Tags - Documents | Metadata Tags - Messages | Metadata Tags - Files |
|---------------|---------------------------|--------------------------|-----------------------|
| `FileName`    | `Language`                | `From`                   | `FileName`            |
| `FilePath`    | `StartPage`               | `To`                     | `FileExtension`       |
| `FileSize`    | `EndPage`                 | `CC`                     | `FileSize`            |
| `Hash`        | `ReviewComment`           | `BCC`                    | `DateCreated`         |
|               |                           | `Subject`                | `DateAccessed`        |
|               |                           | `Header`                 | `DateModified`        |
|               |                           | `DateSent`               | `DatePrinted`         |
|               |                           | `DateReceived`           | `Title`               |
|               |                           | `HasAttachments`         | `Subject`             |
|               |                           | `AttachmentCount`        | `Author`              |
|               |                           | `AttachmentNames`        | `Company`             |
|               |                           | `ReadFlag`               | `Category`            |
|               |                           | `ImportanceFlag`         | `Keywords`            |
|               |                           | `MessageClass`           | `Comments`            |
|               |                           | `FlagStatus`             |                       |
