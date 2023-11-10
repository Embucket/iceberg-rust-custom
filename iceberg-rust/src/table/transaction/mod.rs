/*!
 * Defines the [Transaction] type that performs multiple [Operation]s with ACID properties.
*/

use futures::StreamExt;

use crate::{
    catalog::tabular::Tabular,
    error::Error,
    spec::{manifest::DataFile, schema::Schema},
    table::Table,
    util::strip_prefix,
};

use self::operation::Operation;

mod operation;

/// Transactions let you perform a sequence of [Operation]s that can be committed to be performed with ACID guarantees.
pub struct TableTransaction<'table> {
    table: &'table mut Table,
    operations: Vec<Operation>,
    branch: Option<String>,
}

impl<'table> TableTransaction<'table> {
    /// Create a transaction for the given table.
    pub fn new(table: &'table mut Table, branch: Option<&str>) -> Self {
        TableTransaction {
            table,
            operations: vec![],
            branch: branch.map(ToString::to_string),
        }
    }
    /// Update the schmema of the table
    pub fn update_schema(mut self, schema: Schema) -> Self {
        self.operations.push(Operation::UpdateSchema(schema));
        self
    }
    /// Update the spec of the table
    pub fn update_spec(mut self, spec_id: i32) -> Self {
        self.operations.push(Operation::UpdateSpec(spec_id));
        self
    }
    /// Quickly append files to the table
    pub fn append(mut self, files: Vec<DataFile>) -> Self {
        self.operations.push(Operation::NewAppend {
            branch: self.branch.clone(),
            files,
        });
        self
    }
    /// Update the properties of the table
    pub fn update_properties(mut self, entries: Vec<(String, String)>) -> Self {
        self.operations.push(Operation::UpdateProperties(entries));
        self
    }
    /// Update the snapshot summary of the table
    pub fn update_snapshot_summary(mut self, entries: Vec<(String, String)>) -> Self {
        self.operations.push(Operation::UpdateSnapshotSummary {
            branch: self.branch.clone(),
            entries,
        });
        self
    }
    /// Commit the transaction to perform the [Operation]s with ACID guarantees.
    pub async fn commit(self) -> Result<(), Error> {
        let object_store = self.table.object_store();
        let catalog = self.table.catalog();
        let identifier = self.table.identifier.clone();
        let branch = self.branch;

        // Before executing the transactions operations, update the metadata for a new snapshot
        self.table.increment_sequence_number();
        if self.operations.iter().any(|op| match op {
            Operation::NewAppend {
                branch: _,
                files: _,
            } => true,
            _ => false,
        }) {
            self.table.new_snapshot(branch).await?;
        }
        // Execute the table operations
        let table = futures::stream::iter(self.operations)
            .fold(
                Ok::<&mut Table, Error>(self.table),
                |table, op| async move {
                    let table = table?;
                    op.execute(table).await?;
                    Ok(table)
                },
            )
            .await?;
        // Write the new state to the object store

        let metadata_json = serde_json::to_string(&table.metadata())?;
        let metadata_file_location = table.new_metadata_location()?;
        object_store
            .put(
                &strip_prefix(&metadata_file_location).into(),
                metadata_json.into(),
            )
            .await?;
        let previous_metadata_file_location = table.metadata_location();
        if let Tabular::Table(new_table) = catalog
            .clone()
            .update_table(
                identifier,
                metadata_file_location.as_ref(),
                previous_metadata_file_location,
            )
            .await?
        {
            *table = new_table;
            Ok(())
        } else {
            Err(Error::InvalidFormat(
                "Entity returned from catalog".to_string(),
            ))
        }
    }
}
