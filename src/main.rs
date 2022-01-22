// use std::time;
// use druid::PlatformError;
// use rdkafka::config::RDKafkaLogLevel;
// use std::str;
//
// use crate::gui::main_window;
// use crate::kafka::KafkaOps;
//
// mod gui;
// mod kafka;
//
// fn main() -> Result<(), PlatformError> {
//     env_logger::init();
//
//     let db: sled::Db = sled::open("my_db").unwrap();
//     db.insert("test", "test");
//
//     let value = db.get("test").unwrap();
//     if let Some(v) = value {
//         let s = str::from_utf8(v.as_ref()).unwrap();
//         println!("value: {}", s);
//     }
//
//     let ko = KafkaOps::builder()
//         .with_prop("enable.auto.commit", "false")
//         .with_bootstrap("10.53.91.21:9093")
//         .with_ssl_cert_verification(false)
//         .with_ssl_keystore_location("/Users/sbt-sementsov-av/projects/certs/kafka.p12")
//         .with_ssl_keystore_password("123456")
//         .with_security_protocol("SSL")
//         .with_group_id("rust-grp-2")
//         .with_message_max_bytes(100)
//         .with_fetch_max_bytes(102400)
//         .open(RDKafkaLogLevel::Info)
//         .expect("Error creating kafka ops");
//
//     let consumer = ko.create_consumer().expect("Error creating consumer");
//     let metadata = consumer.read_topic_metadata("message-from-product").unwrap();
//     let topic = &metadata.topics()[0];
//     for p in topic.partitions() {
//         println!("partition: {}: {}, {:?}, {:?}", p.id(), p.leader(), p.isr(), p.replicas());
//     }
//
//     let (start, end) = consumer.read_watermarks("message-from-product", 0).unwrap();
//     println!("Partition: {} to {}", start, end);
//
//     // compute start offset
//     let msgs = consumer.read_from_partition_tail(
//         "message-from-product",
//         0,
//         10,
//         time::Duration::from_secs(5))
//         .expect("Error reading messages");
//
//     println!("{:?}", msgs.len());
//     main_window()
// }

// let der = fs::read("/Users/sbt-sementsov-av/projects/certs/kafka.p12").unwrap();
// let ks = Pkcs12::from_der(&der).unwrap();
// let parsed_ks = ks.parse("123456").unwrap();
// let mut store_builder = X509StoreBuilder::new().unwrap();
// let stack = parsed_ks.chain.unwrap();
// for cert in stack {
//     store_builder.add_cert(cert);
// }
// let store = store_builder.build();
//
// let mut builder = SslConnector::builder(SslMethod::tls_client())
//     .unwrap();
// builder.set_cert_store(store);
// builder.set_verify(SslVerifyMode::NONE);
// builder.set_mode(SslMode::all());
// builder.set_keylog_callback(|k, name| {
//    println!("{:?} - {}", k, name);
// });
// let ssl_con = builder.build();
//
// let security = SecurityConfig::new(ssl_con);
//
// let mut consumer =
//     Consumer::from_hosts(vec!("10.53.91.21:9092".to_owned()))
//         .with_security(security)
//         .with_topic_partitions("__consumer_offsets".to_owned(), &[0])
//         .with_fallback_offset(FetchOffset::Earliest)
//         .with_group("my-group".to_owned())
//         .with_offset_storage(GroupOffsetStorage::Zookeeper)
//         .create()
//         .unwrap();
// loop {
//     for ms in consumer.poll().unwrap().iter() {
//         for m in ms.messages() {
//             println!("{:?}", m);
//         }
//         consumer.consume_messageset(ms);
//     }
//     consumer.commit_consumed().unwrap();
// }

mod ice;
mod kafka;

use iced::{Application, Settings};
use ice::MainWindow;

pub fn main() -> iced::Result {
    env_logger::init();
    MainWindow::run(Settings::default())
}