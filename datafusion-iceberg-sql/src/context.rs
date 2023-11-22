use std::{collections::HashMap, sync::Arc};

use arrow_schema::DataType;
use datafusion_common::{config::ConfigOptions, DataFusionError, TableReference};
use datafusion_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use datafusion_sql::planner::ContextProvider;
use iceberg_rust::catalog::{identifier::Identifier, Catalog};

use crate::IcebergTableSource;

pub struct IcebergContext {
    sources: HashMap<String, Arc<dyn TableSource>>,
    config_options: ConfigOptions,
}

impl IcebergContext {
    pub async fn new(
        tables: Vec<(String, Identifier)>,
        catalogs: &HashMap<String, Arc<dyn Catalog>>,
        branch: Option<&str>,
    ) -> Result<IcebergContext, DataFusionError> {
        let mut sources = HashMap::new();
        for (catalog_name, identifier) in tables {
            let catalog = catalogs
                .get(&catalog_name)
                .ok_or(DataFusionError::Internal(format!(
                    "Catalog {} was not provided",
                    &catalog_name
                )))?;
            let tabular = catalog
                .clone()
                .load_table(&identifier)
                .await
                .map_err(|err| DataFusionError::Internal(err.to_string()))?;
            let table_source = IcebergTableSource::new(tabular, branch);
            sources.insert(
                catalog_name + "." + &identifier.namespace().to_string() + "." + &identifier.name(),
                Arc::new(table_source) as Arc<dyn TableSource>,
            );
        }
        let config_options = ConfigOptions::default();
        Ok(IcebergContext {
            sources,
            config_options,
        })
    }
}

impl ContextProvider for IcebergContext {
    fn get_table_source(
        &self,
        name: TableReference,
    ) -> Result<Arc<dyn TableSource>, DataFusionError> {
        match name {
            TableReference::Full {
                catalog,
                schema,
                table,
            } => self
                .sources
                .get(&(catalog.to_string() + "." + &schema + "." + &table))
                .cloned()
                .ok_or(DataFusionError::Internal(format!(
                    "Couldn't resolve table reference {}.{}",
                    &schema, &table
                ))),
            _ => Err(DataFusionError::Internal(
                "Only partial table refence supported".to_string(),
            )),
        }
    }
    fn get_function_meta(&self, _name: &str) -> Option<Arc<ScalarUDF>> {
        None
    }
    fn get_variable_type(&self, _variable_names: &[String]) -> Option<DataType> {
        None
    }
    fn get_aggregate_meta(&self, _name: &str) -> Option<Arc<AggregateUDF>> {
        None
    }
    fn get_window_meta(&self, _name: &str) -> Option<Arc<WindowUDF>> {
        None
    }
    fn options(&self) -> &ConfigOptions {
        &self.config_options
    }
}
