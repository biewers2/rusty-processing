# file-processing

### Structure

* type
  * subtype
    * mod.rs
    * PROCESSOR.rs - Extracts embedded files, and (based on specified properties) extracts metadata and text
    * text.rs (?) -> <file>.text.txt
    * metadata.rs (?) -> <file>.metadata.json
    * pdf.rs (?) -> render.pdf

https://edrm.net/wiki/3-0text-metadata-and-image-extraction/

### Metadata

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
