/*!
 * A Struct for the view metadata   
*/

use std::collections::HashMap;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::schema::Schema;

use _serde::ViewMetadataEnum;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(try_from = "ViewMetadataEnum", into = "ViewMetadataEnum")]
/// Fields for the version 1 of the view metadata.
pub struct ViewMetadata {
    /// An integer version number for the view format; must be 1
    pub format_version: FormatVersion,
    /// The view’s base location. This is used to determine where to store manifest files and view metadata files.
    pub location: String,
    ///	Current version of the view. Set to ‘1’ when the view is first created.
    pub current_version_id: i64,
    /// An array of structs describing the last known versions of the view. Controlled by the table property: “version.history.num-entries”. See section Versions.
    pub versions: HashMap<i64, Version>,
    /// A list of timestamp and version ID pairs that encodes changes to the current version for the view.
    /// Each time the current-version-id is changed, a new entry should be added with the last-updated-ms and the new current-version-id.
    pub version_log: Vec<VersionLogStruct>,
    /// A string to string map of view properties. This is used for metadata such as “comment” and for settings that affect view maintenance.
    /// This is not intended to be used for arbitrary metadata.
    pub properties: Option<HashMap<String, String>>,
    ///	A list of schemas, the same as the ‘schemas’ field from Iceberg table spec.
    pub schemas: Option<HashMap<i32, Schema>>,
    ///	ID of the current schema of the view
    pub current_schema_id: Option<i32>,
}

impl ViewMetadata {
    /// Get current schema
    #[inline]
    pub fn current_schema(&self) -> Result<&Schema, anyhow::Error> {
        self.schemas
            .as_ref()
            .and_then(|schema| {
                self.current_schema_id
                    .and_then(|schema_id| schema.get(&schema_id))
            })
            .ok_or_else(|| anyhow!("Schema not found"))
    }
    /// Get current version
    #[inline]
    pub fn current_version(&self) -> Result<&Version, anyhow::Error> {
        self.versions
            .get(&self.current_version_id)
            .ok_or_else(|| anyhow!("Version {} not found", self.current_version_id))
    }
}

mod _serde {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use crate::model::{schema::SchemaV2, table_metadata::VersionNumber};

    use super::{FormatVersion, Version, VersionLogStruct, ViewMetadata};

    /// Metadata of an iceberg view
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(untagged)]
    pub(super) enum ViewMetadataEnum {
        /// Version 1 of the table metadata
        V1(ViewMetadataV1),
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "kebab-case")]
    /// Fields for the version 1 of the view metadata.
    pub struct ViewMetadataV1 {
        /// An integer version number for the view format; must be 1
        pub format_version: VersionNumber<1>,
        /// The view’s base location. This is used to determine where to store manifest files and view metadata files.
        pub location: String,
        ///	Current version of the view. Set to ‘1’ when the view is first created.
        pub current_version_id: i64,
        /// An array of structs describing the last known versions of the view. Controlled by the table property: “version.history.num-entries”. See section Versions.
        pub versions: Vec<Version>,
        /// A list of timestamp and version ID pairs that encodes changes to the current version for the view.
        /// Each time the current-version-id is changed, a new entry should be added with the last-updated-ms and the new current-version-id.
        pub version_log: Vec<VersionLogStruct>,
        /// A string to string map of view properties. This is used for metadata such as “comment” and for settings that affect view maintenance.
        /// This is not intended to be used for arbitrary metadata.
        pub properties: Option<HashMap<String, String>>,
        ///	A list of schemas, the same as the ‘schemas’ field from Iceberg table spec.
        pub schemas: Option<Vec<SchemaV2>>,
        ///	ID of the current schema of the view
        pub current_schema_id: Option<i32>,
    }

    impl TryFrom<ViewMetadataEnum> for ViewMetadata {
        type Error = anyhow::Error;
        fn try_from(value: ViewMetadataEnum) -> Result<Self, Self::Error> {
            match value {
                ViewMetadataEnum::V1(metadata) => metadata.try_into(),
            }
        }
    }

    impl From<ViewMetadata> for ViewMetadataEnum {
        fn from(value: ViewMetadata) -> Self {
            match value.format_version {
                FormatVersion::V1 => ViewMetadataEnum::V1(value.into()),
            }
        }
    }

    impl TryFrom<ViewMetadataV1> for ViewMetadata {
        type Error = anyhow::Error;
        fn try_from(value: ViewMetadataV1) -> Result<Self, Self::Error> {
            Ok(ViewMetadata {
                format_version: FormatVersion::V1,
                location: value.location,
                current_version_id: value.current_version_id,
                versions: HashMap::from_iter(value.versions.into_iter().map(|x| (x.version_id, x))),
                version_log: value.version_log,
                properties: value.properties,
                schemas: match value.schemas {
                    Some(schemas) => Some(HashMap::from_iter(
                        schemas
                            .into_iter()
                            .map(|x| Ok((x.schema_id, x.try_into()?)))
                            .collect::<Result<Vec<_>, anyhow::Error>>()?,
                    )),
                    None => None,
                },
                current_schema_id: value.current_schema_id,
            })
        }
    }

    impl From<ViewMetadata> for ViewMetadataV1 {
        fn from(value: ViewMetadata) -> Self {
            ViewMetadataV1 {
                format_version: VersionNumber::<1>,
                location: value.location,
                current_version_id: value.current_version_id,
                versions: value.versions.into_values().collect(),
                version_log: value.version_log,
                properties: value.properties,
                schemas: value
                    .schemas
                    .map(|schemas| schemas.into_values().map(|x| x.into()).collect()),
                current_schema_id: value.current_schema_id,
            }
        }
    }
}
#[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone)]
#[repr(u8)]
/// Iceberg format version
pub enum FormatVersion {
    /// Iceberg spec version 1
    V1 = b'1',
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
/// Fields for the version 2 of the view metadata.
pub struct Version {
    /// Monotonically increasing id indicating the version of the view. Starts with 1.
    pub version_id: i64,
    ///	Timestamp expressed in ms since epoch at which the version of the view was created.
    pub timestamp_ms: i64,
    /// A string map summarizes the version changes, including operation, described in Summary.
    pub summary: Summary,
    /// A list of “representations” as described in Representations.
    pub representations: Vec<Representation>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
/// Fields for the version 2 of the view metadata.
pub struct VersionLogStruct {
    ///	The timestamp when the referenced version was made the current version
    pub timestamp_ms: i64,
    /// Version id of the view
    pub version_id: i64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// View operation that create the metadata file
pub enum Operation {
    /// Create view
    Create,
    /// Replace view
    Replace,
}

/// Serialize for PrimitiveType wit special handling for
/// Decimal and Fixed types.
impl Serialize for Operation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use Operation::*;
        match self {
            Create => serializer.serialize_str("create"),
            Replace => serializer.serialize_str("replace"),
        }
    }
}

/// Serialize for PrimitiveType wit special handling for
/// Decimal and Fixed types.
impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "create" {
            Ok(Operation::Create)
        } else if s == "replace" {
            Ok(Operation::Replace)
        } else {
            Err(serde::de::Error::custom("Invalid view operation."))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
/// Fields for the version 2 of the view metadata.
pub struct Summary {
    /// A string value indicating the view operation that caused this metadata to be created. Allowed values are “create” and “replace”.
    pub operation: Operation,
    /// A string value indicating the version of the engine that performed the operation
    pub engine_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case", tag = "type")]
/// Fields for the version 2 of the view metadata.
pub enum Representation {
    #[serde(rename = "sql")]
    /// This type of representation stores the original view definition in SQL and its SQL dialect.
    Sql {
        /// A string representing the original view definition in SQL
        sql: String,
        /// A string specifying the dialect of the ‘sql’ field. It can be used by the engines to detect the SQL dialect.
        dialect: String,
        /// ID of the view’s schema when the version was created
        schema_id: Option<i64>,
        /// A string specifying the catalog to use when the table or view references in the view definition do not contain an explicit catalog.
        default_catalog: Option<String>,
        /// The namespace to use when the table or view references in the view definition do not contain an explicit namespace.
        /// Since the namespace may contain multiple parts, it is serialized as a list of strings.
        default_namespace: Option<Vec<String>>,
        /// A list of strings of field aliases optionally specified in the create view statement.
        /// The list should have the same length as the schema’s top level fields. See the example below.
        field_aliases: Option<Vec<String>>,
        /// A list of strings of field comments optionally specified in the create view statement.
        /// The list should have the same length as the schema’s top level fields. See the example below.
        field_docs: Option<Vec<String>>,
    },
}

#[cfg(test)]
mod tests {

    use anyhow::Result;

    use crate::model::view_metadata::ViewMetadata;

    #[test]
    fn test_deserialize_view_data_v1() -> Result<()> {
        let data = r#"
        {
            "format-version" : 1,
            "location" : "s3n://my_company/my/warehouse/anorwood.db/common_view",
            "current-version-id" : 1,
            "properties" : { 
              "comment" : "View captures all the data from the table"
            },
            "versions" : [ {
              "version-id" : 1,
              "parent-version-id" : -1,
              "timestamp-ms" : 1573518431292,
              "summary" : {
                "operation" : "create",
                "engineVersion" : "presto-350"
              },
              "representations" : [ {
                "type" : "sql",
                "sql" : "SELECT *\nFROM\n  base_tab\n",
                "dialect" : "presto",
                "schema-id" : 1,
                "default-catalog" : "iceberg",
                "default-namespace" : [ "anorwood" ]
              } ]
            } ],
            "version-log" : [ {
              "timestamp-ms" : 1573518431292,
              "version-id" : 1
            } ],
            "schemas": [ {
              "schema-id": 1,
              "type" : "struct",
              "fields" : [ {
                "id" : 0,
                "name" : "c1",
                "required" : false,
                "type" : "int",
                "doc" : ""
              }, {
                "id" : 1,
                "name" : "c2",
                "required" : false,
                "type" : "string",
                "doc" : ""
              } ]
            } ],
            "current-schema-id": 1
          }
        "#;
        let metadata =
            serde_json::from_str::<ViewMetadata>(data).expect("Failed to deserialize json");
        //test serialise deserialise works.
        let metadata_two: ViewMetadata = serde_json::from_str(
            &serde_json::to_string(&metadata).expect("Failed to serialize metadata"),
        )
        .expect("Failed to serialize json");
        dbg!(&metadata, &metadata_two);
        assert_eq!(metadata, metadata_two);

        Ok(())
    }
}
