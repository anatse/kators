use crate::kafka::consumer_ops::ConsumerOps;
use crate::kafka::{AdminOps, KafkaOpsError, ProducerOps};
use anyhow::Result;
use rdkafka::admin::AdminClient;
use rdkafka::client::DefaultClientContext;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::producer::BaseProducer;
use rdkafka::ClientConfig;
use std::collections::HashMap;

const BOOTSTRAP: &str = "bootstrap.servers";
const CLIENT_ID: &str = "client.id";
const GROUP_ID: &str = "group.id";

pub type Pref = HashMap<String, String>;

pub struct KafkaOps {
    preferences: Pref,
    client_config: Option<ClientConfig>,
}

impl KafkaOps {
    pub fn builder() -> KafkaOpsBuilder {
        KafkaOpsBuilder {
            preferences: HashMap::new(),
        }
    }

    pub fn create_consumer(&self) -> Result<ConsumerOps> {
        match &self.client_config {
            None => Err(KafkaOpsError::ClientNotOpen.into()),
            Some(client) => client.create().map(ConsumerOps::new).map_err(|e| e.into()),
        }
    }

    pub fn create_producer(&self) -> Result<ProducerOps> {
        match &self.client_config {
            None => Err(KafkaOpsError::ClientNotOpen.into()),
            Some(client) => client
                .create::<BaseProducer>()
                .map(ProducerOps::new)
                .map_err(|e| e.into()),
        }
    }

    pub fn create_admin(&self) -> Result<AdminOps> {
        match &self.client_config {
            None => Err(KafkaOpsError::ClientNotOpen.into()),
            Some(client) => client
                .create::<AdminClient<DefaultClientContext>>()
                .map(AdminOps::new)
                .map_err(|e| e.into()),
        }
    }
}

pub struct KafkaOpsBuilder {
    preferences: Pref,
}

impl KafkaOpsBuilder {
    pub fn with_prop<T: AsRef<str>>(mut self, prop_name: T, prop_value: T) -> Self {
        self.preferences.insert(
            prop_name.as_ref().to_string(),
            prop_value.as_ref().to_string(),
        );
        self
    }

    pub fn with_bootstrap<T: AsRef<str>>(mut self, bootstrap: T) -> Self {
        self.preferences
            .insert(BOOTSTRAP.to_string(), bootstrap.as_ref().to_string());
        self
    }

    pub fn with_client_id<T: AsRef<str>>(mut self, client_id: T) -> Self {
        self.preferences
            .insert(CLIENT_ID.to_string(), client_id.as_ref().to_string());
        self
    }

    pub fn with_group_id<T: AsRef<str>>(mut self, group_id: T) -> Self {
        self.preferences
            .insert(GROUP_ID.to_string(), group_id.as_ref().to_string());
        self
    }

    pub fn with_security_protocol<T: AsRef<str>>(mut self, protocol: T) -> Self {
        self.preferences.insert(
            "security.protocol".to_string(),
            protocol.as_ref().to_string(),
        );
        self
    }

    pub fn with_ssl_cert_verification(mut self, flag: bool) -> Self {
        self.preferences.insert(
            "enable.ssl.certificate.verification".to_string(),
            flag.to_string(),
        );
        self
    }

    pub fn with_ssl_keystore_location<T: AsRef<str>>(mut self, location: T) -> Self {
        self.preferences.insert(
            "ssl.keystore.location".to_string(),
            location.as_ref().to_string(),
        );
        self
    }

    pub fn with_ssl_keystore_password<T: AsRef<str>>(mut self, password: T) -> Self {
        self.preferences.insert(
            "ssl.keystore.password".to_string(),
            password.as_ref().to_string(),
        );
        self
    }

    pub fn with_message_max_bytes(mut self, size: u64) -> Self {
        self.preferences
            .insert("queued.max.messages.kbytes".to_string(), size.to_string());
        self
    }

    pub fn with_fetch_max_bytes(mut self, size: u64) -> Self {
        self.preferences
            .insert("fetch.message.max.bytes".to_string(), size.to_string());
        self
    }

    pub fn open(&self, log_level: RDKafkaLogLevel) -> Result<KafkaOps> {
        let mut client_config = ClientConfig::new();
        client_config.set_log_level(log_level);

        for (k, v) in &self.preferences {
            client_config.set(k, v);
        }

        Ok(KafkaOps {
            preferences: self.preferences.clone(),
            client_config: Some(client_config),
        })
    }
}
