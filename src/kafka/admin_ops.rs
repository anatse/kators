use std::future::Future;
use rdkafka::admin::{AdminClient, AdminOptions, NewPartitions, NewTopic, TopicReplication, TopicResult};
use rdkafka::client::DefaultClientContext;
use anyhow::Result;

pub struct AdminOps {
    admin: AdminClient<DefaultClientContext>,
}

impl AdminOps {
    pub fn new(admin: AdminClient<DefaultClientContext>) -> Self {
        Self {
            admin,
        }
    }

    pub async fn create_topic<T: AsRef<str>>(&self, name: T, num_partitions: i32, replication_factor: i32) -> Result<Vec<TopicResult>> {
        let topic = NewTopic::new(name.as_ref(), num_partitions, TopicReplication::Fixed(replication_factor));
        self.admin.create_topics(&[topic], &AdminOptions::default())
            .await
            .map_err(|c| c.into())
    }

    pub async fn delete_topic<T: AsRef<str>>(&self, name: T) -> Result<Vec<TopicResult>> {
        self.admin.delete_topics(&[name.as_ref()], &AdminOptions::default())
            .await
            .map_err(|c| c.into())
    }

    pub async fn add_partition<T: AsRef<str>>(&self, topic: T, num_partitions: usize) -> Result<Vec<TopicResult>> {
        let parts = NewPartitions::new(topic.as_ref(), num_partitions);
        self.admin.create_partitions(&[parts], &AdminOptions::default())
            .await
            .map_err(|c| c.into())
    }
}