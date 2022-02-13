use anyhow::Result;
use futures::executor;
use log::{error, info};
use rdkafka::admin::{
    AdminClient, AdminOptions, NewPartitions, NewTopic, TopicReplication, TopicResult,
};
use rdkafka::client::DefaultClientContext;
use std::fmt::{Debug, Formatter};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

pub struct AdminOps {
    admin: AdminClient<DefaultClientContext>,
}

impl Debug for AdminOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("AdminOps(..)")
    }
}

#[derive(Clone, Debug)]
pub enum AdminCommand {
    CreateTopic(String, i32, i32),
    DeleteTopic(String),
    AddPartition(String, i32),
    Stop,
}

impl AdminOps {
    pub fn new(admin: AdminClient<DefaultClientContext>) -> Self {
        Self { admin }
    }

    pub fn start_worker(self) -> Sender<AdminCommand> {
        let (tx, rx) = mpsc::channel::<AdminCommand>();
        thread::spawn(move || {
            info!("Start worker");
            while let Ok(cmd) = rx.recv() {
                info!("Received command: {:?}", cmd);
                match cmd {
                    AdminCommand::CreateTopic(name, partitions, replication_factor) => {
                        match executor::block_on(self.create_topic(
                            name,
                            partitions,
                            replication_factor,
                        )) {
                            Ok(res) => info!("Topic created: {:?}", res),
                            Err(err) => error!("Error creating topic: {}", err),
                        }
                    }
                    AdminCommand::DeleteTopic(name) => {
                        match executor::block_on(self.delete_topic(name)) {
                            Ok(res) => info!("Topic deleted: {:?}", res),
                            Err(err) => error!("Error deleting topic: {}", err),
                        }
                    }
                    AdminCommand::AddPartition(name, num_partitions) => {
                        match executor::block_on(self.add_partition(name, num_partitions as usize))
                        {
                            Ok(res) => info!("Partition added: {:?}", res),
                            Err(err) => error!("Error adding partitions: {}", err),
                        }
                    }
                    AdminCommand::Stop => break,
                }
            }

            info!("Finish worker");
        });

        tx
    }

    pub async fn create_topic<T: AsRef<str>>(
        &self,
        name: T,
        num_partitions: i32,
        replication_factor: i32,
    ) -> Result<Vec<TopicResult>> {
        let topic = NewTopic::new(
            name.as_ref(),
            num_partitions,
            TopicReplication::Fixed(replication_factor),
        );
        self.admin
            .create_topics(&[topic], &AdminOptions::default())
            .await
            .map_err(|c| c.into())
    }

    pub async fn delete_topic<T: AsRef<str>>(&self, name: T) -> Result<Vec<TopicResult>> {
        self.admin
            .delete_topics(&[name.as_ref()], &AdminOptions::default())
            .await
            .map_err(|c| c.into())
    }

    pub async fn add_partition<T: AsRef<str>>(
        &self,
        topic: T,
        num_partitions: usize,
    ) -> Result<Vec<TopicResult>> {
        let parts = NewPartitions::new(topic.as_ref(), num_partitions);
        self.admin
            .create_partitions(&[parts], &AdminOptions::default())
            .await
            .map_err(|c| c.into())
    }
}
