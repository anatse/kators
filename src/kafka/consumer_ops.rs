use std::cmp::max;
use std::{cmp, time};
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::metadata::Metadata;
use anyhow::Result;
use log::{error, trace};
use rdkafka::message::OwnedMessage;
use rdkafka::{Offset, TopicPartitionList};

pub struct ConsumerOps {
    consumer: BaseConsumer,
}

impl ConsumerOps {
    pub fn new(consumer: BaseConsumer) -> Self {
        Self {
            consumer,
        }
    }

    pub fn read_metadata(&self) -> Result<Metadata> {
        self.consumer.fetch_metadata(None, time::Duration::from_millis(1000))
            .map_err(|e| e.into())
    }

    pub fn read_topic_metadata<T: AsRef<str>>(&self, topic: T) -> Result<Metadata> {
        self.consumer.fetch_metadata(Some(topic.as_ref()), time::Duration::from_millis(1000))
            .map_err(|e| e.into())
    }

    pub fn read_watermarks<T: AsRef<str>>(&self, topic: T, partition: i32) -> Result<(i64, i64)> {
        self.consumer.fetch_watermarks(topic.as_ref(), partition, time::Duration::from_millis(1000))
            .map_err(|e| e.into())
    }

    fn read_from_consumer(&self, n_last_messages: i64, timeout: time::Duration) -> Result<Vec<OwnedMessage>> {
        let start = time::Instant::now();
        let poll_timeout = time::Duration::from_millis(10);
        let mut buffer = Vec::new();

        while time::Instant::elapsed(&start) < timeout && buffer.len() < n_last_messages as usize {
            trace!("start polling: {} elapsed: {:?}", buffer.len(), time::Instant::elapsed(&start));

            match self.consumer.poll(poll_timeout) {
                None => if !buffer.is_empty() {
                    break;
                },
                Some(Err(e)) => error!("Error while receiving message {:?}", e),
                Some(Ok(msg)) => {
                    buffer.push(msg.detach());
                }
            };
        }

        Ok(buffer)
    }

    pub fn read_from_partition_tail<T: AsRef<str>>(&self, topic: T, partition: i32, n_last_messages: i64, timeout: time::Duration) -> Result<Vec<OwnedMessage>> {
        // Compute available offset
        let (start, end) = self.read_watermarks(topic.as_ref(), partition)?;
        let offset = cmp::min(end - start, n_last_messages);
        let mut tnp = TopicPartitionList::new();
        tnp.add_partition_offset(topic.as_ref(), partition, Offset::OffsetTail(offset));
        // Assign topic partition
        self.consumer.assign(&tnp)?;
        self.read_from_consumer(n_last_messages, timeout)
    }

    pub fn read_from_partition_offset<T: AsRef<str>>(&self, topic: T, partition: i32, offset: i64, max_messages: i64, timeout: time::Duration) -> Result<Vec<OwnedMessage>> {
        let mut tnp = TopicPartitionList::new();
        tnp.add_partition_offset(topic.as_ref(), partition, Offset::Offset(offset));
        // Assign topic partition
        self.consumer.assign(&tnp)?;
        self.read_from_consumer(max_messages, timeout)
    }

    pub fn read_from_partition<T: AsRef<str>>(&self, topic: T, partition: i32, max_messages: i64, timeout: time::Duration) -> Result<Vec<OwnedMessage>> {
        let mut tnp = TopicPartitionList::new();
        tnp.add_partition_offset(topic.as_ref(), partition, Offset::Beginning);
        // Assign topic partition
        self.consumer.assign(&tnp)?;
        self.read_from_consumer(max_messages, timeout)
    }
}