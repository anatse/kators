use rdkafka::message::{OwnedHeaders, ToBytes};
use rdkafka::producer::{BaseProducer, BaseRecord};
use anyhow::Result;

pub struct ProducerOps {
    producer: BaseProducer,
}

impl ProducerOps {
    pub fn new(producer: BaseProducer) -> Self {
        Self {
            producer,
        }
    }

    pub fn send<T: AsRef<str>, K: ToBytes, P: ToBytes>(
        &self,
        topic: T,
        partition: i32,
        key: &K,
        payload: &P,
        headers: Vec<(T, T)>
    ) -> Result<()> {
        let owned_headers = headers.iter().fold(OwnedHeaders::new(), |hdr, (k, v)| {
            hdr.add(k.as_ref(), v.as_ref())
        });

        let rec = BaseRecord::to(topic.as_ref())
            .partition(partition)
            .key(key)
            .headers(owned_headers)
            .payload(payload);

         self.producer.send(rec).map_err(|(e, _)| e.into())
    }
}