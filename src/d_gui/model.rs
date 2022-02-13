use rdkafka::metadata::{MetadataBroker, MetadataTopic};
use std::fmt::{Display, Formatter};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct KafkaTreeBroker {
    server_name: String,
    id: i32,
    host: String,
    port: i32,
}

impl Display for KafkaTreeBroker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{}:{}", self.host, self.port).as_str())
    }
}

impl KafkaTreeBroker {
    pub fn from_md(server_name: String, broker: &MetadataBroker) -> Self {
        Self {
            server_name,
            id: broker.id(),
            host: broker.host().to_string(),
            port: broker.port(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct KafkaTreePartition {
    pub server_name: String,
    pub topic_name: String,
    pub id: i32,
    pub leader: i32,
}

impl Display for KafkaTreePartition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{}", self.id).as_str())
    }
}

#[derive(Debug, Clone)]
pub struct KafkaTreeTopic {
    pub server_name: String,
    pub name: String,
    pub partitions: Vec<KafkaTreePartition>,
}

impl Display for KafkaTreeTopic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl PartialEq for KafkaTreeTopic {
    fn eq(&self, other: &Self) -> bool {
        self.server_name == other.server_name && self.name == other.name
    }
}

impl Eq for KafkaTreeTopic {}

impl KafkaTreeTopic {
    pub fn from_md<T: AsRef<str>>(server_name: T, topic: &MetadataTopic) -> Self {
        let topic_name = topic.name().to_string();
        Self {
            server_name: server_name.as_ref().to_string(),
            name: topic_name.clone(),
            partitions: topic
                .partitions()
                .iter()
                .map(|p| KafkaTreePartition {
                    server_name: server_name.as_ref().to_string(),
                    topic_name: topic_name.clone(),
                    id: p.id(),
                    leader: p.leader(),
                })
                .collect(),
        }
    }

    pub fn partitions(&self) -> &Vec<KafkaTreePartition> {
        &self.partitions
    }
}
