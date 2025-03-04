// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

use std::any::Any;
use std::sync::Arc;

use common_datablocks::DataBlock;
use common_datavalues::DataField;
use common_datavalues::DataSchemaRef;
use common_datavalues::DataSchemaRefExt;
use common_datavalues::DataType;
use common_datavalues::StringArray;
use common_datavalues::UInt16Array;
use common_datavalues::UInt8Array;
use common_exception::Result;
use common_planners::Partition;
use common_planners::ReadDataSourcePlan;
use common_planners::ScanPlan;
use common_planners::Statistics;
use common_streams::DataBlockStream;
use common_streams::SendableDataBlockStream;

use crate::datasources::ITable;
use crate::sessions::FuseQueryContextRef;

pub struct ClustersTable {
    schema: DataSchemaRef,
}

impl ClustersTable {
    pub fn create() -> Self {
        ClustersTable {
            schema: DataSchemaRefExt::create(vec![
                DataField::new("name", DataType::Utf8, false),
                DataField::new("host", DataType::Utf8, false),
                DataField::new("port", DataType::UInt16, false),
                DataField::new("priority", DataType::UInt8, false),
            ]),
        }
    }
}

#[async_trait::async_trait]
impl ITable for ClustersTable {
    fn name(&self) -> &str {
        "clusters"
    }

    fn engine(&self) -> &str {
        "SystemClusters"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> Result<DataSchemaRef> {
        Ok(self.schema.clone())
    }

    fn is_local(&self) -> bool {
        true
    }

    fn read_plan(
        &self,
        _ctx: FuseQueryContextRef,
        scan: &ScanPlan,
        _partitions: usize,
    ) -> Result<ReadDataSourcePlan> {
        Ok(ReadDataSourcePlan {
            db: "system".to_string(),
            table: self.name().to_string(),
            schema: self.schema.clone(),
            partitions: vec![Partition {
                name: "".to_string(),
                version: 0,
            }],
            statistics: Statistics::default(),
            description: "(Read from system.clusters table)".to_string(),
            scan_plan: Arc::new(scan.clone()),
        })
    }

    async fn read(&self, ctx: FuseQueryContextRef) -> Result<SendableDataBlockStream> {
        let nodes = ctx.try_get_cluster()?.get_nodes()?;
        let names: Vec<&str> = nodes.iter().map(|x| x.name.as_str()).collect();
        let hosts = nodes
            .iter()
            .map(|x| x.address.hostname())
            .collect::<Vec<_>>();
        let hostnames = hosts.iter().map(|x| x.as_str()).collect::<Vec<&str>>();
        let ports: Vec<u16> = nodes.iter().map(|x| x.address.port()).collect();
        let priorities: Vec<u8> = nodes.iter().map(|x| x.priority).collect();
        let block = DataBlock::create_by_array(self.schema.clone(), vec![
            Arc::new(StringArray::from(names)),
            Arc::new(StringArray::from(hostnames)),
            Arc::new(UInt16Array::from(ports)),
            Arc::new(UInt8Array::from(priorities)),
        ]);
        Ok(Box::pin(DataBlockStream::create(
            self.schema.clone(),
            None,
            vec![block],
        )))
    }
}
