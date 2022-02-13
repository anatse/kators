use crate::d_gui::model::KafkaTreeTopic;
use crate::kafka::{AdminCommand, AdminOps, ConsumerOps, KafkaServer, ProducerOps};
use anyhow::Result as AnyResult;
use eframe::egui;
use eframe::egui::Ui;
use futures::executor;
use log::{error, info};
use rdkafka::admin::TopicReplication;
use rdkafka::message::{Headers, OwnedMessage};
use rdkafka::statistics::Topic;
use rdkafka::Message;
use serde::{Deserialize, Serialize};
use sled::Tree as DbTree;
use std::fmt::{Display, Formatter, Write};
use std::rc::Rc;
use std::str::{from_utf8, from_utf8_unchecked};
use std::sync::mpsc::Sender;
use std::time;
use tracing_subscriber::fmt::format;

#[derive(Debug, PartialEq, Copy, Clone)]
enum InfoPanel {
    Properties,
    Data,
}

impl Display for InfoPanel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InfoPanel::Properties => f.write_str("Properties"),
            InfoPanel::Data => f.write_str("Data"),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum MessagesOffset {
    FromTail,
    FromStart,
    FromOffset,
}

impl Display for MessagesOffset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MessagesOffset::FromTail => f.write_str("Tail"),
            MessagesOffset::FromStart => f.write_str("Start"),
            MessagesOffset::FromOffset => f.write_str("Offset"),
        }
    }
}

#[derive(Debug)]
pub struct TopicInfo {
    db: Rc<DbTree>,
    db_topics: Rc<DbTree>,
    server: KafkaServer,
    topic: KafkaTreeTopic,
    selected_panel: InfoPanel,
    partition_len: i32,
    replication_factor: i32,
    topic_pref: TopicPref,
    filter: String,
    offset_type: MessagesOffset,
    offset: i32,
    partition: i32,
    ops: Option<TopicOps>,
    data: Vec<OwnedMessage>,
    selected_data: usize,
}

struct KMsg(OwnedMessage);
impl PartialEq for KMsg {
    fn eq(&self, other: &Self) -> bool {
        self.0.partition() == other.0.partition() && self.0.timestamp() == other.0.timestamp()
    }
}

#[derive(Debug)]
struct TopicOps {
    consumer: ConsumerOps,
    producer: ProducerOps,
    admin: Sender<AdminCommand>,
}

impl TopicOps {
    pub fn from_server(server: &KafkaServer) -> AnyResult<Self> {
        let kafka_ops = server.open()?;
        Ok(Self {
            consumer: kafka_ops.create_consumer()?,
            producer: kafka_ops.create_producer()?,
            admin: kafka_ops.create_admin()?.start_worker(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TopicPref {
    key: String,
    key_format: String,
    data_format: String,
    last_messages: u32,
}

impl TopicPref {
    pub fn from_db<T: AsRef<str>>(name: T, db: &DbTree) -> Self {
        let empty = Self {
            key: name.as_ref().to_string(),
            key_format: "Binary".to_string(),
            data_format: "Binary".to_string(),
            last_messages: 200,
        };

        match db.get(name.as_ref()) {
            Ok(data) => match data {
                None => empty,
                Some(data) => match bson::from_reader(data.as_ref()) {
                    Ok(res) => res,
                    Err(_) => empty,
                },
            },
            Err(_) => empty,
        }
    }

    pub fn store(&self, db: &DbTree) {
        match bson::to_vec(&self) {
            Ok(data) => {
                let _ = db.insert(&self.key, data);
            }
            Err(_) => {}
        }
    }
}

impl TopicInfo {
    pub fn new(db: Rc<DbTree>, db_topics: Rc<DbTree>) -> Self {
        Self {
            db,
            db_topics,
            server: KafkaServer::default(),
            topic: KafkaTreeTopic {
                server_name: "".to_string(),
                name: "".to_string(),
                partitions: vec![],
            },
            selected_panel: InfoPanel::Properties,
            partition_len: 0,
            replication_factor: -1,
            topic_pref: TopicPref {
                key: "default".to_string(),
                key_format: "Binary".to_string(),
                data_format: "Binary".to_string(),
                last_messages: 200,
            },
            filter: "".to_string(),
            offset_type: MessagesOffset::FromTail,
            offset: 0,
            partition: -1,
            ops: None,
            data: vec![],
            selected_data: 1,
        }
    }

    /// Sends command to admin kafka interface. This is oneway operation
    /// To check result see application logs
    pub fn send_admin_command(&self, cmd: AdminCommand) {
        if let Some(ops) = &self.ops {
            ops.admin.send(cmd);
        }
    }

    pub fn set_topic(&mut self, topic: &KafkaTreeTopic) -> &mut Self {
        if &self.topic != topic {
            self.topic = topic.clone();
            self.partition_len = topic.partitions.len() as i32;
            self.topic_pref = TopicPref::from_db(
                format!("{}:{}", topic.server_name, topic.name).as_str(),
                &self.db_topics,
            );
        }

        if self.server.name != self.topic.server_name {
            info!("{}", &topic.server_name);
            match KafkaServer::from_db(&self.db, &self.topic.server_name) {
                Ok(server) => {
                    self.server = server;
                    // fill topic ops
                    self.ops = TopicOps::from_server(&self.server).ok();
                }
                Err(err) => {
                    error!(
                        "Error loading server {} from preferences: {}",
                        &self.topic.server_name, err
                    );
                }
            }
        }

        self
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            [InfoPanel::Properties, InfoPanel::Data]
                .into_iter()
                .for_each(|s| {
                    ui.selectable_value(&mut self.selected_panel, s, s.to_string());
                })
        });
        ui.separator();

        ui.with_layout(
            egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
            |ui| match self.selected_panel {
                InfoPanel::Properties => self.show_properties(ui, false),
                InfoPanel::Data => self.show_data(ui),
            },
        );
    }

    pub fn show_properties(&mut self, ui: &mut Ui, enabled: bool) {
        if !enabled {
            if self.ops.is_some() {
                ui.horizontal(|ui| {
                    if ui.button("Remove").clicked() {
                        if let Some(ops) = &self.ops {
                            ops.admin
                                .send(AdminCommand::DeleteTopic(self.topic.name.clone()));
                        }
                    }
                    if ui.button("Add partition").clicked() {
                        if let Some(ops) = &self.ops {
                            let partitions = self.topic.partitions.len() + 1;
                            ops.admin.send(AdminCommand::AddPartition(
                                self.topic.name.clone(),
                                partitions as i32,
                            ));
                        }
                    }
                });
                ui.separator();
            }
        }

        egui::Grid::new("topic_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Name");
                ui.add_enabled(enabled, egui::TextEdit::singleline(&mut self.topic.name));
                ui.end_row();

                ui.label("Partitions");
                ui.add_enabled(enabled, egui::DragValue::new(&mut self.partition_len));
                ui.end_row();

                ui.label("Replication factor");
                ui.add_enabled(enabled, egui::DragValue::new(&mut self.replication_factor));
                ui.end_row();

                if !enabled {
                    ui.separator();
                    ui.end_row();

                    ui.label("Key");
                    egui::ComboBox::from_id_source("key")
                        .selected_text(&self.topic_pref.key_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.topic_pref.key_format,
                                "Binary".to_string(),
                                "Binary".to_string(),
                            );
                            ui.selectable_value(
                                &mut self.topic_pref.key_format,
                                "String".to_string(),
                                "String".to_string(),
                            );
                        });
                    ui.end_row();

                    ui.label("Data");
                    egui::ComboBox::from_id_source("data")
                        .selected_text(&self.topic_pref.data_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.topic_pref.data_format,
                                "Binary".to_string(),
                                "Binary".to_string(),
                            );
                            ui.selectable_value(
                                &mut self.topic_pref.data_format,
                                "String".to_string(),
                                "String".to_string(),
                            );
                        });
                    ui.end_row();

                    ui.label("Max messages");
                    ui.add(egui::DragValue::new(&mut self.topic_pref.last_messages));
                    ui.end_row();
                    ui.separator();
                    ui.end_row();

                    if ui.button("Update").clicked() {
                        self.topic_pref.store(&self.db_topics);
                    }
                    ui.end_row();
                }
            });
    }

    fn read_data(&mut self) -> AnyResult<Vec<OwnedMessage>> {
        if let Some(ops) = &self.ops {
            let timeout = time::Duration::from_secs(1);
            let data = match self.offset_type {
                MessagesOffset::FromTail => {
                    if self.partition == -1 {
                        // read from all partitions
                        self.topic.partitions.iter().fold(vec![], |mut v, p| {
                            match ops.consumer.read_from_partition_tail(
                                &self.topic.name,
                                p.id,
                                self.topic_pref.last_messages as i64,
                                timeout,
                            ) {
                                Ok(data) => {
                                    v.extend(data);
                                    v
                                }
                                Err(_) => v,
                            }
                        })
                    } else {
                        ops.consumer.read_from_partition_tail(
                            &self.topic.name,
                            self.partition,
                            self.topic_pref.last_messages as i64,
                            timeout,
                        )?
                    }
                }
                MessagesOffset::FromStart => {
                    vec![]
                }
                MessagesOffset::FromOffset => {
                    vec![]
                }
            };

            Ok(data)
        } else {
            Ok(vec![])
        }
    }

    pub fn show_data(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("refresh").clicked() {
                match self.read_data() {
                    Ok(data) => {
                        self.data = data;
                    }
                    Err(err) => {
                        self.data.clear();
                        error!("Error reading data: {}", err)
                    }
                }
            }

            ui.separator();
            ui.label("filter");
            ui.text_edit_singleline(&mut self.filter);

            ui.label("partition");
            egui::ComboBox::from_id_source("partitions")
                .selected_text(if self.partition >= 0 {
                    self.partition.to_string()
                } else {
                    " ".to_string()
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.partition, -1, " ");
                    for i in 0..self.partition_len {
                        ui.selectable_value(
                            &mut self.partition,
                            i,
                            if i >= 0 {
                                i.to_string()
                            } else {
                                " ".to_string()
                            },
                        );
                    }
                });

            ui.label("offset");
            egui::ComboBox::from_id_source("offset_type")
                .selected_text(&self.offset_type.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.offset_type,
                        MessagesOffset::FromTail,
                        MessagesOffset::FromTail.to_string(),
                    );
                    ui.selectable_value(
                        &mut self.offset_type,
                        MessagesOffset::FromStart,
                        MessagesOffset::FromStart.to_string(),
                    );
                    if self.partition != -1 {
                        ui.selectable_value(
                            &mut self.offset_type,
                            MessagesOffset::FromOffset,
                            MessagesOffset::FromOffset.to_string(),
                        );
                    }
                });

            if self.offset_type == MessagesOffset::FromOffset {
                // Read offset
                if let Some(ops) = &self.ops {
                    let (s, e) = ops
                        .consumer
                        .read_watermarks(&self.topic.name, self.partition)
                        .ok()
                        .unwrap_or((0, 0));
                    ui.label(format!("Range: {} - {}: ", s, e));
                }
                ui.add(egui::DragValue::new(&mut self.offset));
            }
        });

        ui.vertical_centered_justified(|ui| {
            egui::TopBottomPanel::top("data_list")
                .resizable(true)
                .height_range(300.0..=800.0)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("my_rows_grid")
                            .striped(true)
                            .min_col_width(80.0)
                            .max_col_width(600.0)
                            .show(ui, |ui| {
                                let mut index = 0;
                                for d in &self.data {
                                    index += 1;
                                    ui.selectable_value(
                                        &mut self.selected_data,
                                        index,
                                        index.to_string(),
                                    );
                                    self.draw_data_row(ui, &d);
                                }
                            });
                    });
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.selected_data != 0 && self.selected_data <= self.data.len() {
                            if self.topic_pref.data_format == "String" {
                                let s = self.data[self.selected_data - 1]
                                    .payload()
                                    .and_then(|m| from_utf8(m).ok())
                                    .unwrap_or("");
                                let mut text = s.to_string();
                                ui.add(
                                    egui::TextEdit::multiline(&mut text)
                                        // .font(egui::TextStyle::Monospace) // for cursor height
                                        .code_editor()
                                        .desired_rows(10)
                                        .lock_focus(true)
                                        .desired_width(f32::INFINITY),
                                );
                            }
                        }
                    });
            });
        });
    }

    fn draw_data_row(&self, ui: &mut Ui, msg: &OwnedMessage) {
        ui.label(msg.partition().to_string());
        if self.topic_pref.key_format == "String" {
            let s = msg.key().and_then(|m| from_utf8(m).ok()).unwrap_or("");
            ui.label(s);
        } else {
            ui.label("[hex]...");
        }

        if self.topic_pref.data_format == "String" {
            let s = msg.payload().and_then(|m| from_utf8(m).ok()).unwrap_or("");
            let s = if s.len() > 100 {
                s.replace("\n", " ")[..100].to_string()
            } else {
                s.to_string()
            };
            ui.label(s);
        } else {
            ui.label("[hex]...");
        }

        // ui.label(msg.headers().map(|hdr|
        //     for i in 0..hdr.count() {
        //         let h = hdr.get(i);
        //     }
        // ));

        ui.end_row();
    }
}
